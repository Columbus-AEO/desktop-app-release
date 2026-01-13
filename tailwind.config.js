/** @type {import('tailwindcss').Config} */
export default {
  content: [
    './index.html',
    './src/**/*.{vue,js,ts,jsx,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        brand: {
          DEFAULT: '#6366f1',
          dark: '#4f46e5',
          light: '#818cf8',
        },
      },
      animation: {
        'spin-slow': 'spin 1.5s linear infinite',
        'pulse-subtle': 'pulse 1.5s ease-in-out infinite',
      },
    },
  },
  plugins: [],
}
