# macOS Installation Workaround

## English

### If you see: "Columbus is damaged and cannot be opened"

This is a temporary issue with the current macOS build. We're working on properly signing the app, but in the meantime, here's how to run it:

#### Quick Fix (One Command)

Open **Terminal** (Applications → Utilities → Terminal) and run:

```bash
xattr -rd com.apple.quarantine /Applications/Columbus.app
```

Then try opening Columbus again - it should work!

---

## Deutsch

### Wenn die Meldung erscheint: "Columbus ist beschädigt und kann nicht geöffnet werden"

Dies ist ein temporäres Problem mit dem aktuellen macOS-Build. Wir arbeiten an einer ordnungsgemäß signierten Version, aber in der Zwischenzeit können Sie die App so starten:

#### Schnelle Lösung (Ein Befehl)

Öffnen Sie das **Terminal** (Programme → Dienstprogramme → Terminal) und führen Sie aus:

```bash
xattr -rd com.apple.quarantine /Applications/Columbus.app
```

Versuchen Sie dann erneut, Columbus zu öffnen - es sollte funktionieren!

---

### What This Does / Was macht das?

**EN**: This removes the "quarantine" flag that macOS adds to apps downloaded from the internet. It's safe to do this for Columbus.

**DE**: Dies entfernt die "Quarantäne"-Markierung, die macOS bei aus dem Internet heruntergeladenen Apps hinzufügt. Dies ist für Columbus sicher.

### Why Is This Needed? / Warum ist das nötig?

**EN**: macOS requires apps to be properly code-signed and notarized by Apple. The current build is missing proper signing. We're adding this in the next release!

**DE**: macOS erfordert, dass Apps ordnungsgemäß von Apple signiert und notarisiert werden. Der aktuelle Build hat keine korrekte Signierung. Wir fügen dies in der nächsten Version hinzu!

### Alternative: Right-Click Method / Alternative: Rechtsklick-Methode

**EN**:
1. Right-click on **Columbus.app** in Applications
2. Select **Open**
3. Click **Open** in the warning dialog

**DE**:
1. Rechtsklick auf **Columbus.app** in Programme
2. **Öffnen** auswählen
3. In der Warnung auf **Öffnen** klicken

This sometimes works but may not work for all users.
Dies funktioniert manchmal, aber möglicherweise nicht für alle Benutzer.

---

**Note / Hinweis**: Future versions will be properly signed and won't require this workaround. / Zukünftige Versionen werden ordnungsgemäß signiert sein und diesen Workaround nicht benötigen.
