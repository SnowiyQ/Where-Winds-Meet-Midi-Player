import defaultTheme from 'tailwindcss/defaultTheme';

/** @type {import('tailwindcss').Config} */
export default {
  content: [
    './index.html',
    './src/**/*.{svelte,js,ts,jsx,tsx}',
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'Segoe UI', 'system-ui', '-apple-system', 'BlinkMacSystemFont', ...defaultTheme.fontFamily.sans],
      },
      colors: {
        'overlay-bg': 'rgba(15, 23, 42, 0.85)',
        'overlay-border': 'rgba(100, 116, 139, 0.3)',
        spotify: {
          green: '#1db954',
          neon: '#1ed760',
          dark: '#121212',
          darker: '#0a0a0a',
          gray: '#181818',
        },
      },
      backgroundImage: {
        'spotify-gradient': 'linear-gradient(135deg, #1db954 0%, #1ed760 100%)',
      },
      backdropBlur: {
        xs: '2px',
      },
      keyframes: {
        musicBar1: {
          '0%, 100%': { height: '60%' },
          '50%': { height: '20%' },
        },
        musicBar2: {
          '0%, 100%': { height: '100%' },
          '50%': { height: '40%' },
        },
        musicBar3: {
          '0%, 100%': { height: '80%' },
          '50%': { height: '30%' },
        },
        pulseGlow: {
          '0%, 100%': { boxShadow: '0 0 0 0 rgba(29, 185, 84, 0.4)' },
          '50%': { boxShadow: '0 0 0 8px rgba(29, 185, 84, 0)' },
        },
        fadeIn: {
          from: { opacity: '0', transform: 'translateY(10px)' },
          to: { opacity: '1', transform: 'translateY(0)' },
        },
        slideUp: {
          from: { opacity: '0', transform: 'translateY(20px)' },
          to: { opacity: '1', transform: 'translateY(0)' },
        },
        slideDown: {
          from: { opacity: '0', transform: 'translateY(-20px)' },
          to: { opacity: '1', transform: 'translateY(0)' },
        },
        scaleIn: {
          from: { transform: 'scale(0.95)', opacity: '0' },
          to: { transform: 'scale(1)', opacity: '1' },
        },
        shimmer: {
          '0%': { backgroundPosition: '-200% 0' },
          '100%': { backgroundPosition: '200% 0' },
        },
      },
      animation: {
        'music-bar-1': 'musicBar1 0.6s ease-in-out infinite',
        'music-bar-2': 'musicBar2 0.8s ease-in-out infinite',
        'music-bar-3': 'musicBar3 0.7s ease-in-out infinite',
        'pulse-glow': 'pulseGlow 2s ease-in-out infinite',
        'fade-in': 'fadeIn 0.3s ease-out forwards',
        'slide-up': 'slideUp 0.3s ease-out forwards',
        'slide-down': 'slideDown 0.3s ease-out forwards',
        'scale-in': 'scaleIn 0.2s ease-out forwards',
        shimmer: 'shimmer 1.5s infinite',
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
      },
    },
  },
  plugins: [],
};