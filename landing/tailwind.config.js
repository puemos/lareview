/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./index.html"],
  theme: {
    extend: {
      colors: {
        "velura-light": "#EBE6DE",
        "velura-bg": "#EBE6DE",
        "velura-dark": "#1A1A1A",
        "velura-gray": "#5A5A5A",
      },
      fontFamily: {
        sans: ["Inter", "sans-serif"],
        display: ["Space Grotesk", "sans-serif"],
      },
    },
  },
  plugins: [],
};
