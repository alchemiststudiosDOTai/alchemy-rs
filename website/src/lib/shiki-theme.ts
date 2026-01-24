import type { ThemeRegistration } from "shiki";

export const alchemyDark: ThemeRegistration = {
  name: "alchemy-dark",
  type: "dark",
  colors: {
    "editor.background": "#000000",
    "editor.foreground": "#e0e0e0",
  },
  settings: [
    {
      settings: {
        foreground: "#e0e0e0",
      },
    },
    {
      scope: ["comment", "punctuation.definition.comment"],
      settings: {
        foreground: "#6b6b6b",
        fontStyle: "italic",
      },
    },
    {
      scope: ["string", "string.quoted", "string.template"],
      settings: {
        foreground: "#c9a26d",
      },
    },
    {
      scope: ["constant.numeric", "constant.language.boolean"],
      settings: {
        foreground: "#d4976c",
      },
    },
    {
      scope: ["keyword", "keyword.control", "storage.type", "storage.modifier"],
      settings: {
        foreground: "#c45a2c",
      },
    },
    {
      scope: ["entity.name.function", "support.function"],
      settings: {
        foreground: "#e0e0e0",
      },
    },
    {
      scope: [
        "entity.name.type",
        "entity.name.class",
        "support.type",
        "support.class",
      ],
      settings: {
        foreground: "#d4976c",
      },
    },
    {
      scope: ["variable", "variable.other"],
      settings: {
        foreground: "#e0e0e0",
      },
    },
    {
      scope: ["variable.parameter"],
      settings: {
        foreground: "#c9a26d",
      },
    },
    {
      scope: ["punctuation", "meta.brace"],
      settings: {
        foreground: "#808080",
      },
    },
    {
      scope: ["entity.name.tag"],
      settings: {
        foreground: "#c45a2c",
      },
    },
    {
      scope: ["entity.other.attribute-name"],
      settings: {
        foreground: "#c9a26d",
      },
    },
    {
      scope: ["support.type.primitive", "keyword.type"],
      settings: {
        foreground: "#d4976c",
      },
    },
    {
      scope: ["constant.language", "constant.other"],
      settings: {
        foreground: "#c45a2c",
      },
    },
    {
      scope: ["meta.macro", "entity.name.function.macro"],
      settings: {
        foreground: "#c45a2c",
      },
    },
    {
      scope: ["variable.language.self", "variable.language.this"],
      settings: {
        foreground: "#c45a2c",
        fontStyle: "italic",
      },
    },
    {
      scope: ["keyword.operator"],
      settings: {
        foreground: "#808080",
      },
    },
    {
      scope: ["entity.name.namespace", "entity.name.module"],
      settings: {
        foreground: "#e0e0e0",
      },
    },
  ],
};

export const alchemyLight: ThemeRegistration = {
  name: "alchemy-light",
  type: "light",
  colors: {
    "editor.background": "#ffffff",
    "editor.foreground": "#1a1a1a",
  },
  settings: [
    {
      settings: {
        foreground: "#1a1a1a",
      },
    },
    {
      scope: ["comment", "punctuation.definition.comment"],
      settings: {
        foreground: "#8b8b8b",
        fontStyle: "italic",
      },
    },
    {
      scope: ["string", "string.quoted", "string.template"],
      settings: {
        foreground: "#986c2e",
      },
    },
    {
      scope: ["constant.numeric", "constant.language.boolean"],
      settings: {
        foreground: "#9a5518",
      },
    },
    {
      scope: ["keyword", "keyword.control", "storage.type", "storage.modifier"],
      settings: {
        foreground: "#b5421a",
      },
    },
    {
      scope: ["entity.name.function", "support.function"],
      settings: {
        foreground: "#1a1a1a",
      },
    },
    {
      scope: [
        "entity.name.type",
        "entity.name.class",
        "support.type",
        "support.class",
      ],
      settings: {
        foreground: "#9a5518",
      },
    },
    {
      scope: ["variable", "variable.other"],
      settings: {
        foreground: "#1a1a1a",
      },
    },
    {
      scope: ["variable.parameter"],
      settings: {
        foreground: "#986c2e",
      },
    },
    {
      scope: ["punctuation", "meta.brace"],
      settings: {
        foreground: "#5c5c5c",
      },
    },
    {
      scope: ["entity.name.tag"],
      settings: {
        foreground: "#b5421a",
      },
    },
    {
      scope: ["entity.other.attribute-name"],
      settings: {
        foreground: "#986c2e",
      },
    },
    {
      scope: ["support.type.primitive", "keyword.type"],
      settings: {
        foreground: "#9a5518",
      },
    },
    {
      scope: ["constant.language", "constant.other"],
      settings: {
        foreground: "#b5421a",
      },
    },
    {
      scope: ["meta.macro", "entity.name.function.macro"],
      settings: {
        foreground: "#b5421a",
      },
    },
    {
      scope: ["variable.language.self", "variable.language.this"],
      settings: {
        foreground: "#b5421a",
        fontStyle: "italic",
      },
    },
    {
      scope: ["keyword.operator"],
      settings: {
        foreground: "#5c5c5c",
      },
    },
    {
      scope: ["entity.name.namespace", "entity.name.module"],
      settings: {
        foreground: "#1a1a1a",
      },
    },
  ],
};
