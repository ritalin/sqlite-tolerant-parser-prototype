import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
  resolve: {
    alias: {
      'pkg/parser': path.resolve(__dirname, 'pkg/parser'),
    }
  }
});
