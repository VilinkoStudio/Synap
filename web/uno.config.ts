import { defineConfig, presetIcons, presetUno, presetTypography } from 'unocss';

export default defineConfig({
  presets: [
    presetUno(),
    presetTypography(),
    presetIcons({
      scale: 1.1
    })
  ],
  theme: {
    colors: {
      paper: '#f4efe2',
      ink: '#16120d',
      accent: '#b14d2e'
    }
  }
});
