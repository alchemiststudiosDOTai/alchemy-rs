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
    "editor.foreground": "#24292e",
  },
  settings: [
    {
      settings: {
        foreground: "#24292e",
      },
    },
    {
      scope: ["comment", "punctuation.definition.comment"],
      settings: {
        foreground: "#6a737d",
        fontStyle: "italic",
      },
    },
    {
      scope: ["string", "string.quoted", "string.template"],
      settings: {
        foreground: "#22863a",
      },
    },
    {
      scope: ["constant.numeric", "constant.language.boolean"],
      settings: {
        foreground: "#005cc5",
      },
    },
    {
      scope: ["keyword", "keyword.control", "storage.type", "storage.modifier"],
      settings: {
        foreground: "#d73a49",
      },
    },
    {
      scope: ["entity.name.function", "support.function"],
      settings: {
        foreground: "#6f42c1",
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
        foreground: "#e36209",
      },
    },
    {
      scope: ["variable", "variable.other"],
      settings: {
        foreground: "#24292e",
      },
    },
    {
      scope: ["variable.parameter"],
      settings: {
        foreground: "#e36209",
      },
    },
    {
      scope: ["punctuation", "meta.brace"],
      settings: {
        foreground: "#24292e",
      },
    },
    {
      scope: ["entity.name.tag"],
      settings: {
        foreground: "#22863a",
      },
    },
    {
      scope: ["entity.other.attribute-name"],
      settings: {
        foreground: "#6f42c1",
      },
    },
    {
      scope: ["support.type.primitive", "keyword.type"],
      settings: {
        foreground: "#e36209",
      },
    },
    {
      scope: ["constant.language", "constant.other"],
      settings: {
        foreground: "#005cc5",
      },
    },
    {
      scope: ["meta.macro", "entity.name.function.macro"],
      settings: {
        foreground: "#6f42c1",
      },
    },
    {
      scope: ["variable.language.self", "variable.language.this"],
      settings: {
        foreground: "#d73a49",
        fontStyle: "italic",
      },
    },
    {
      scope: ["keyword.operator"],
      settings: {
        foreground: "#d73a49",
      },
    },
    {
      scope: ["entity.name.namespace", "entity.name.module"],
      settings: {
        foreground: "#6f42c1",
      },
    },
  ],
};
