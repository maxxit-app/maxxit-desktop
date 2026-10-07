import js from "@eslint/js";
import tseslint from "typescript-eslint";
import globals from "globals";
export default tseslint.config(
  { ignores: ["dist/**", "node_modules/**", "src-tauri/**"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: { globals: globals.browser },
    rules: { "@typescript-eslint/no-explicit-any": "off" },
  },
  { files: ["scripts/**/*.mjs"], languageOptions: { globals: globals.node } },
);
