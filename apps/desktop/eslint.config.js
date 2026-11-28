import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

// Security rules from the desktop master plan, section 7.4: data from the
// system is always rendered as text, and nothing evaluates strings as code.
const security = {
    "no-eval": "error",
    "no-implied-eval": "error",
    "no-new-func": "error",
    "no-restricted-syntax": [
        "error",
        {
            selector: "JSXAttribute[name.name='dangerouslySetInnerHTML']",
            message: "Render system data as text. dangerouslySetInnerHTML is not allowed.",
        },
        {
            selector: "MemberExpression[property.name=/^(innerHTML|outerHTML)$/]",
            message: "Do not write HTML strings into the DOM.",
        },
        {
            selector: "CallExpression[callee.property.name='insertAdjacentHTML']",
            message: "Do not write HTML strings into the DOM.",
        },
    ],
};

export default tseslint.config(
    { ignores: ["dist", "src-tauri", "node_modules"] },
    js.configs.recommended,
    ...tseslint.configs.strictTypeChecked,
    {
        files: ["**/*.{ts,tsx}"],
        languageOptions: {
            globals: globals.browser,
            parserOptions: {
                projectService: true,
                tsconfigRootDir: import.meta.dirname,
            },
        },
        plugins: { "react-hooks": reactHooks },
        rules: {
            ...reactHooks.configs.recommended.rules,
            ...security,
        },
    },
    {
        files: ["eslint.config.js"],
        ...tseslint.configs.disableTypeChecked,
    },
);
