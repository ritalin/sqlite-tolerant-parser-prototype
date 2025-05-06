import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
  resolve: {
    alias: {
      'pkg/scanner': path.resolve(__dirname, 'pkg/scanner'),
    }
  }
});
