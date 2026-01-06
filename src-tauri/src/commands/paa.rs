use crate::{storage, webview::WebviewManager, AppState};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex as TokioMutex;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PAAQuestion {
    pub question_text: String,
    pub snippet: Option<String>,
    pub source_url: Option<String>,
    pub depth: i32,
    pub parent_index: Option<usize>,
}

#[derive(Clone, Serialize)]
pub struct PAAProgressEvent {
    pub phase: String,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PAADiscoveryResponse {
    pub success: bool,
    pub discovery_run_id: Option<String>,
    pub questions_found: Option<usize>,
    pub questions_inserted: Option<usize>,
    pub duplicates_filtered: Option<usize>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub code: Option<String>,
}

#[derive(Clone, Deserialize, Debug)]
struct PAACheckResponse {
    allowed: Option<bool>,
    message: Option<String>,
    #[serde(rename = "resetAt")]
    reset_at: Option<String>,
    error: Option<String>,
}

/// Start PAA (People Also Ask) keyword discovery for a product
#[tauri::command]
pub async fn start_paa_discovery(
    product_id: String,
    seed_keyword: String,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<PAADiscoveryResponse, String> {
    eprintln!("[PAA Discovery] Starting for product {} with keyword: {}", product_id, seed_keyword);

    // Ensure we have a valid auth token
    let token = crate::commands::auth::ensure_valid_token(&state).await?;

    // Check if google_aio is authenticated - PAA requires logged-in Google context
    let instance_id = storage::get_active_instance_id();
    let google_auth = if instance_id.is_empty() {
        storage::get_country_platform_auth("local", "google_aio")
    } else {
        storage::get_instance_country_platform_auth(&instance_id, "local", "google_aio")
    };

    let is_google_authenticated = google_auth.map(|a| a.is_authenticated).unwrap_or(false);

    if !is_google_authenticated {
        return Ok(PAADiscoveryResponse {
            success: false,
            discovery_run_id: None,
            questions_found: None,
            questions_inserted: None,
            duplicates_filtered: None,
            message: Some("Please authenticate Google AI Overview first. Go to Manage Auth and log into Google.".to_string()),
            error: Some("Google not authenticated".to_string()),
            code: Some("GOOGLE_AUTH_REQUIRED".to_string()),
        });
    }

    // Check rate limit BEFORE starting the expensive scraping process
    eprintln!("[PAA Discovery] Checking rate limit...");
    let client = reqwest::Client::new();
    let check_response = client
        .get(format!("{}/functions/v1/paa-discovery-check?productId={}", crate::SUPABASE_URL, product_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("apikey", crate::SUPABASE_ANON_KEY)
        .send()
        .await
        .map_err(|e| format!("Failed to check rate limit: {}", e))?;

    let check_status = check_response.status();
    let check_body = check_response.text().await.map_err(|e| format!("Failed to read check response: {}", e))?;

    eprintln!("[PAA Discovery] Rate limit check response: {} - {}", check_status, check_body);

    if !check_status.is_success() {
        return Ok(PAADiscoveryResponse {
            success: false,
            discovery_run_id: None,
            questions_found: None,
            questions_inserted: None,
            duplicates_filtered: None,
            message: Some("Failed to check rate limit. Please try again.".to_string()),
            error: Some(check_body),
            code: Some("CHECK_FAILED".to_string()),
        });
    }

    let check_result: PAACheckResponse = serde_json::from_str(&check_body)
        .map_err(|e| format!("Failed to parse check response: {}", e))?;

    if check_result.allowed != Some(true) {
        return Ok(PAADiscoveryResponse {
            success: false,
            discovery_run_id: None,
            questions_found: None,
            questions_inserted: None,
            duplicates_filtered: None,
            message: check_result.message,
            error: Some("Rate limit exceeded".to_string()),
            code: Some("RATE_LIMIT_EXCEEDED".to_string()),
        });
    }

    eprintln!("[PAA Discovery] Rate limit check passed, starting scan...");

    // Retry loop - try up to 5 times if no PAA section is found
    let max_attempts = 5;
    let mut questions: Vec<PAAQuestion> = Vec::new();

    for attempt in 1..=max_attempts {
        eprintln!("[PAA Discovery] Attempt {}/{}", attempt, max_attempts);

        // Emit initial progress
        let _ = app.emit("paa:progress", PAAProgressEvent {
            phase: "initializing".to_string(),
            current: 0,
            total: 100,
            message: if attempt == 1 {
                "Opening Google search...".to_string()
            } else {
                format!("Retrying... (attempt {}/{})", attempt, max_attempts)
            },
        });

        // Create webview manager
        let manager = Arc::new(TokioMutex::new(WebviewManager::new()));
        let webview_label = format!("paa-{}", &product_id[..8.min(product_id.len())]);

        eprintln!("[PAA Discovery] Using google_aio authenticated context");

        // Create webview using google_aio data directory (authenticated context)
        let is_visible = cfg!(debug_assertions);
        {
            let mut mgr = manager.lock().await;
            // Use "google_aio" platform key to share authentication with Google AI Overview
            mgr.create_webview_local(&app, &webview_label, "https://www.google.com", is_visible, "google_aio")?;
        }

        // Wait for page to load
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        let _ = app.emit("paa:progress", PAAProgressEvent {
            phase: "searching".to_string(),
            current: 10,
            total: 100,
            message: "Searching Google...".to_string(),
        });

        // Search for the keyword
        let window = app.get_webview_window(&webview_label)
            .ok_or("Webview not found")?;

        let search_script = get_search_script(&seed_keyword);
        window.eval(&search_script).map_err(|e| format!("Search script error: {}", e))?;

        // Wait for search results
        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

        let _ = app.emit("paa:progress", PAAProgressEvent {
            phase: "extracting".to_string(),
            current: 30,
            total: 100,
            message: "Finding People Also Ask questions...".to_string(),
        });

        // Extract PAA questions with recursive expansion
        let extract_script = get_paa_extraction_script();
        window.eval(&extract_script).map_err(|e| format!("Extract script error: {}", e))?;

        // Wait for extraction to complete (includes clicking and expanding)
        // Script has 45 second timeout, so we wait up to 55 seconds
        for i in 0..55 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // Check if extraction is complete
            let url = window.url().map_err(|e| format!("Failed to get URL: {}", e))?;
            let url_str = url.as_str();

            if url_str.contains("#PAA_RESULT:") {
                eprintln!("[PAA Discovery] Extraction complete after {} seconds", i + 1);
                break;
            }

            if i % 5 == 0 {
                let _ = app.emit("paa:progress", PAAProgressEvent {
                    phase: "extracting".to_string(),
                    current: 30 + (i.min(50)),
                    total: 100,
                    message: format!("Expanding questions... ({}s)", i + 1),
                });
            }
        }

        let _ = app.emit("paa:progress", PAAProgressEvent {
            phase: "processing".to_string(),
            current: 80,
            total: 100,
            message: "Processing discovered questions...".to_string(),
        });

        // Read results from URL hash
        let url = window.url().map_err(|e| format!("Failed to get URL: {}", e))?;
        let url_str = url.as_str();

        questions = if let Some(hash_pos) = url_str.find("#PAA_RESULT:") {
            let data = &url_str[hash_pos + 12..];
            match decode_paa_result(data) {
                Ok(qs) => {
                    eprintln!("[PAA Discovery] Decoded {} questions", qs.len());
                    qs
                }
                Err(e) => {
                    eprintln!("[PAA Discovery] Failed to decode results: {}", e);
                    Vec::new()
                }
            }
        } else {
            eprintln!("[PAA Discovery] No PAA results marker found in URL");
            Vec::new()
        };

        // Close webview
        {
            let mut mgr = manager.lock().await;
            mgr.close_webview(&app, &webview_label);
        }

        // If we found questions, break out of retry loop
        if !questions.is_empty() {
            eprintln!("[PAA Discovery] Found {} questions on attempt {}", questions.len(), attempt);
            break;
        }

        // No questions found - retry if we have attempts left
        if attempt < max_attempts {
            eprintln!("[PAA Discovery] No PAA found, will retry in 2 seconds...");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    if questions.is_empty() {
        return Ok(PAADiscoveryResponse {
            success: false,
            discovery_run_id: None,
            questions_found: Some(0),
            questions_inserted: None,
            duplicates_filtered: None,
            message: Some(format!("No 'People Also Ask' section found after {} attempts. Try a different seed keyword, or check that your Google account is properly authenticated.", max_attempts)),
            error: Some("No PAA questions found".to_string()),
            code: Some("NO_PAA_FOUND".to_string()),
        });
    }

    let _ = app.emit("paa:progress", PAAProgressEvent {
        phase: "submitting".to_string(),
        current: 90,
        total: 100,
        message: format!("Submitting {} questions...", questions.len()),
    });

    // Submit to edge function
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/functions/v1/paa-discovery", crate::SUPABASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("apikey", crate::SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "productId": product_id,
            "seedKeyword": seed_keyword,
            "questions": questions.iter().map(|q| {
                serde_json::json!({
                    "questionText": q.question_text,
                    "snippet": q.snippet,
                    "sourceUrl": q.source_url,
                    "depth": q.depth,
                    "parentIndex": q.parent_index
                })
            }).collect::<Vec<_>>()
        }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    let response_text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    eprintln!("[PAA Discovery] API response status: {}, body: {}", status, response_text);

    if !status.is_success() {
        // Try to parse error response
        if let Ok(error_response) = serde_json::from_str::<PAADiscoveryResponse>(&response_text) {
            if error_response.code.as_deref() == Some("RATE_LIMIT_EXCEEDED") {
                return Ok(error_response);
            }
            return Err(error_response.message.or(error_response.error).unwrap_or_else(|| "Unknown error".to_string()));
        }
        return Err(format!("API error: {} - {}", status, response_text));
    }

    let result: PAADiscoveryResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let _ = app.emit("paa:progress", PAAProgressEvent {
        phase: "complete".to_string(),
        current: 100,
        total: 100,
        message: format!("Found {} keywords!", result.questions_inserted.unwrap_or(0)),
    });

    eprintln!("[PAA Discovery] Complete: {:?}", result);
    Ok(result)
}

fn get_search_script(keyword: &str) -> String {
    // URL-encode the keyword for direct navigation
    let encoded_keyword = urlencoding::encode(keyword);

    format!(r#"
        (async function() {{
            console.log('[Columbus PAA] Starting Google search via direct URL...');

            // Handle consent dialog if present (works across all locales)
            const consentBtn = document.querySelector('#L2AGLb, button#L2AGLb');
            if (consentBtn) {{
                console.log('[Columbus PAA] Clicking consent button');
                consentBtn.click();
                await new Promise(r => setTimeout(r, 1500));
            }}

            // Navigate directly to search URL with PAA-friendly parameters
            // nfpr=1 = disable auto-correction (prevents "aeo" -> "seo" etc.)
            // pws=0 = disable personalized web search
            const searchUrl = 'https://www.google.com/search?q={}&nfpr=1&pws=0&source=hp';
            console.log('[Columbus PAA] Navigating to:', searchUrl);
            window.location.href = searchUrl;
        }})();
    "#, encoded_keyword)
}

fn get_paa_extraction_script() -> String {
    r#"
        (async function() {
            console.log('[Columbus PAA] Starting PAA extraction...');
            const questions = [];
            const seenQuestions = new Set();
            const clickedIds = new Set();
            const MAX_QUESTIONS = 50;
            const CLICK_DELAY = 800;
            const MAX_ROUNDS = 15;
            const MAX_TIME_MS = 45000; // 45 second max total time
            const startTime = Date.now();

            // Helper to set result and exit
            const setResultAndExit = () => {
                try {
                    console.log('[Columbus PAA] Setting result with', questions.length, 'questions');
                    const jsonStr = JSON.stringify(questions);
                    const base64 = btoa(unescape(encodeURIComponent(jsonStr)));
                    window.location.hash = 'PAA_RESULT:' + base64;
                    console.log('[Columbus PAA] Result hash set successfully');
                } catch (e) {
                    console.error('[Columbus PAA] Failed to encode result:', e);
                    window.location.hash = 'PAA_RESULT:' + btoa('[]');
                }
            };

            // Check if we've exceeded time limit
            const isTimeUp = () => {
                const elapsed = Date.now() - startTime;
                if (elapsed > MAX_TIME_MS) {
                    console.log('[Columbus PAA] Time limit reached, finishing up...');
                    return true;
                }
                return false;
            };

            // Find the PAA container - MUST have data-initq attribute
            const findPAAContainer = () => {
                // The PAA section always has a container with data-initq
                const container = document.querySelector('div[data-initq]');
                if (container) {
                    console.log('[Columbus PAA] Found PAA container with data-initq');
                    return container;
                }
                // Fallback: look for the cUnQKe class which is the PAA wrapper
                const wrapper = document.querySelector('div.cUnQKe');
                if (wrapper) {
                    console.log('[Columbus PAA] Found PAA container with cUnQKe class');
                    return wrapper;
                }
                return null;
            };

            // Find all PAA question elements ONLY within the PAA container
            const findAllPAAQuestions = (container) => {
                if (!container) return [];

                // Look for related-question-pair elements with data-q inside the container
                let els = container.querySelectorAll('div.related-question-pair[data-q]');
                if (els.length > 0) {
                    console.log('[Columbus PAA] Found', els.length, 'questions with related-question-pair');
                    return Array.from(els);
                }

                // Fallback: div with data-q that has the expandable button structure
                els = container.querySelectorAll('div[data-q]');
                if (els.length > 0) {
                    const filtered = Array.from(els).filter(el => {
                        // Must have the accordion-style expand button
                        return el.querySelector('div[role="button"][aria-expanded]') ||
                               el.querySelector('div.dnXCYb[role="button"]');
                    });
                    console.log('[Columbus PAA] Found', filtered.length, 'questions with data-q + button');
                    return filtered;
                }

                return [];
            };

            // Get a unique ID for an element
            const getElementId = (el) => {
                const q = el.dataset.q || '';
                const lk = el.dataset.lk || '';
                return q + '|' + lk;
            };

            // Extract question text from element
            const extractQuestionText = (el) => {
                if (el.dataset.q) return el.dataset.q;
                const textEl = el.querySelector('span.CSkcDe');
                if (textEl) return textEl.textContent?.trim();
                return null;
            };

            // Extract answer snippet from expanded question
            const extractSnippet = (el) => {
                const expandedContent = el.querySelector('div.wDYxhc');
                if (expandedContent) {
                    return expandedContent.textContent?.trim().substring(0, 500);
                }
                return null;
            };

            // Extract source URL from expanded question
            const extractSourceUrl = (el) => {
                const link = el.querySelector('a.zReHs[href^="http"]');
                if (link) return link.href;
                const anyLink = el.querySelector('a[href^="http"]');
                return anyLink?.href || null;
            };

            // Check if question is expanded
            const isExpanded = (el) => {
                const btn = el.querySelector('[role="button"][aria-expanded]');
                return btn?.getAttribute('aria-expanded') === 'true';
            };

            // Click to expand question
            const expandQuestion = async (el) => {
                const elId = getElementId(el);
                if (isExpanded(el) || clickedIds.has(elId)) return false;

                clickedIds.add(elId);

                const clickTarget = el.querySelector('div[role="button"][aria-expanded]') ||
                                   el.querySelector('div.dnXCYb[role="button"]');

                if (!clickTarget) {
                    console.log('[Columbus PAA] No click target found for:', el.dataset.q?.substring(0, 30));
                    return false;
                }

                clickTarget.click();
                await new Promise(r => setTimeout(r, CLICK_DELAY));
                return true;
            };

            // Main extraction loop
            const extractAllPAA = async (container) => {
                let round = 0;
                let consecutiveEmptyRounds = 0;

                while (round < MAX_ROUNDS && questions.length < MAX_QUESTIONS && !isTimeUp()) {
                    round++;
                    console.log('[Columbus PAA] Round', round, '- Questions so far:', questions.length);

                    const questionEls = findAllPAAQuestions(container);
                    console.log('[Columbus PAA] Found', questionEls.length, 'question elements in container');

                    if (questionEls.length === 0) {
                        consecutiveEmptyRounds++;
                        if (consecutiveEmptyRounds >= 2) {
                            console.log('[Columbus PAA] No questions found for 2 rounds, stopping');
                            break;
                        }
                        await new Promise(r => setTimeout(r, 1000));
                        continue;
                    }

                    consecutiveEmptyRounds = 0;
                    let newQuestionsThisRound = 0;
                    let clickedThisRound = 0;

                    for (const el of questionEls) {
                        if (questions.length >= MAX_QUESTIONS || isTimeUp()) break;

                        const questionText = extractQuestionText(el);
                        if (!questionText) continue;

                        const normalizedText = questionText.toLowerCase().trim();
                        const elId = getElementId(el);

                        // Always try to expand to reveal more questions
                        if (!clickedIds.has(elId)) {
                            const clicked = await expandQuestion(el);
                            if (clicked) clickedThisRound++;
                        }

                        // Skip if we've already recorded this question
                        if (seenQuestions.has(normalizedText)) continue;

                        seenQuestions.add(normalizedText);
                        newQuestionsThisRound++;

                        questions.push({
                            questionText: questionText,
                            snippet: extractSnippet(el),
                            sourceUrl: extractSourceUrl(el),
                            depth: round - 1,
                            parentIndex: null
                        });

                        console.log('[Columbus PAA] Added:', questionText.substring(0, 60));
                    }

                    console.log('[Columbus PAA] Round', round, 'complete. New:', newQuestionsThisRound, 'Clicked:', clickedThisRound);

                    // Stop if nothing happened this round
                    if (newQuestionsThisRound === 0 && clickedThisRound === 0) {
                        console.log('[Columbus PAA] No progress, stopping');
                        break;
                    }

                    // Small delay before next round
                    await new Promise(r => setTimeout(r, 300));
                }

                console.log('[Columbus PAA] Extraction finished. Total:', questions.length, 'Rounds:', round);
            };

            // Scroll to trigger lazy loading
            const triggerLazyLoad = async () => {
                console.log('[Columbus PAA] Triggering lazy load via scroll...');
                for (let i = 0; i < 5; i++) {
                    window.scrollBy(0, 400);
                    await new Promise(r => setTimeout(r, 250));
                }
                window.scrollTo(0, 0);
                await new Promise(r => setTimeout(r, 500));
            };

            // Wait for page to fully load
            await new Promise(r => setTimeout(r, 2000));

            // Try scrolling to trigger lazy-loaded content
            await triggerLazyLoad();
            await new Promise(r => setTimeout(r, 1000));

            // Find the PAA container
            let paaContainer = findPAAContainer();
            console.log('[Columbus PAA] PAA container search after scroll:', !!paaContainer);

            // If not found, try one more scroll
            if (!paaContainer) {
                console.log('[Columbus PAA] No PAA container, trying deeper scroll...');
                window.scrollTo(0, 800);
                await new Promise(r => setTimeout(r, 1500));
                window.scrollTo(0, 0);
                await new Promise(r => setTimeout(r, 1000));
                paaContainer = findPAAContainer();
                console.log('[Columbus PAA] PAA container after deep scroll:', !!paaContainer);
            }

            if (!paaContainer) {
                console.log('[Columbus PAA] No PAA container found on page');
                setResultAndExit();
                return;
            }

            try {
                await extractAllPAA(paaContainer);
            } catch (e) {
                console.error('[Columbus PAA] Error during extraction:', e);
            }

            // Always set result at the end
            setResultAndExit();
        })();
    "#.to_string()
}

fn decode_paa_result(data: &str) -> Result<Vec<PAAQuestion>, String> {
    use std::str;

    // Decode base64 - result is already UTF-8 JSON
    // The JS uses btoa(unescape(encodeURIComponent(json))) which converts UTF-8 to base64
    let decoded = base64_decode(data).map_err(|e| format!("Base64 decode error: {}", e))?;
    let json_str = str::from_utf8(&decoded).map_err(|e| format!("UTF-8 error: {}", e))?;

    // Parse JSON array
    let parsed: Vec<serde_json::Value> = serde_json::from_str(json_str)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let questions = parsed.into_iter()
        .filter_map(|v| {
            Some(PAAQuestion {
                question_text: v.get("questionText")?.as_str()?.to_string(),
                snippet: v.get("snippet").and_then(|s| s.as_str()).map(|s| s.to_string()),
                source_url: v.get("sourceUrl").and_then(|s| s.as_str()).map(|s| s.to_string()),
                depth: v.get("depth").and_then(|d| d.as_i64()).unwrap_or(0) as i32,
                parent_index: v.get("parentIndex").and_then(|p| p.as_u64()).map(|p| p as usize),
            })
        })
        .collect();

    Ok(questions)
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let input = input.trim().replace('-', "+").replace('_', "/");
    let input = input.trim_end_matches('=');

    let mut result = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;

    for c in input.chars() {
        let val = CHARS.iter().position(|&x| x == c as u8)
            .ok_or_else(|| format!("Invalid base64 char: {}", c))? as u32;
        buffer = (buffer << 6) | val;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            result.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }

    Ok(result)
}
