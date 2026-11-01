// Same theme the app used with the Play CDN, now compiled at build time.
// Rebuild with: npx tailwindcss@3.4.17 -c tools/tailwind.config.js -i tools/tailwind.input.css -o ui/vendor/tailwind.css --minify
module.exports = {
  content: ['./ui/index.html', './ui/app.js'],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      colors: {
        sn: {
          bg: '#121214',
          card: '#1b1b1f',
          elevated: '#242429',
          border: '#2e2e36',
          gold: '#FADA5E',
          amber: '#f59e0b',
          green: '#10b981',
          red: '#ef4444',
        },
      },
    },
  },
};
