import { useEffect, useState, useRef } from "react";
import { marked } from "marked";
import { codeToHtml } from "shiki";

interface MarkdownContentProps {
  content: string;
}

// Configure marked to add IDs to headings
const renderer = new marked.Renderer();
renderer.heading = ({ text, depth }) => {
  const id = text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
  return `<h${depth} id="${id}">${text}</h${depth}>`;
};

marked.setOptions({ renderer });

async function highlightCode(html: string): Promise<string> {
  const codeBlockRegex =
    /<pre><code class="language-(\w+)">([\s\S]*?)<\/code><\/pre>/g;
  const matches = [...html.matchAll(codeBlockRegex)];

  if (matches.length === 0) return html;

  let result = html;

  for (const match of matches) {
    const [fullMatch, lang, code] = match;
    const decodedCode = code
      .replace(/&lt;/g, "<")
      .replace(/&gt;/g, ">")
      .replace(/&amp;/g, "&")
      .replace(/&quot;/g, '"')
      .replace(/&#39;/g, "'");

    try {
      const highlighted = await codeToHtml(decodedCode, {
        lang: lang || "text",
        themes: {
          light: "vitesse-light",
          dark: "vitesse-black",
        },
        defaultColor: false,
      });
      // Wrap in a container with copy button
      const wrapped = `<div class="code-block-wrapper">
        <button class="copy-button" aria-label="Copy code" data-code="${encodeURIComponent(decodedCode)}">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="copy-icon"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="check-icon" style="display:none"><path d="M20 6 9 17l-5-5"/></svg>
        </button>
        ${highlighted}
      </div>`;
      result = result.replace(fullMatch, wrapped);
    } catch {
      // If language not supported, keep original
    }
  }

  return result;
}

export function MarkdownContent({ content }: MarkdownContentProps) {
  const [html, setHtml] = useState("");
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const parseMarkdown = async () => {
      const parsed = await marked.parse(content);
      const highlighted = await highlightCode(parsed);
      setHtml(highlighted);
    };
    parseMarkdown();
  }, [content]);

  useEffect(() => {
    if (!containerRef.current) return;

    const handleClick = async (e: MouseEvent) => {
      const button = (e.target as Element).closest(".copy-button");
      if (!button) return;

      const code = decodeURIComponent(button.getAttribute("data-code") || "");
      await navigator.clipboard.writeText(code);

      const copyIcon = button.querySelector(".copy-icon") as HTMLElement;
      const checkIcon = button.querySelector(".check-icon") as HTMLElement;

      if (copyIcon && checkIcon) {
        copyIcon.style.display = "none";
        checkIcon.style.display = "block";

        setTimeout(() => {
          copyIcon.style.display = "block";
          checkIcon.style.display = "none";
        }, 2000);
      }
    };

    containerRef.current.addEventListener("click", handleClick);
    return () => containerRef.current?.removeEventListener("click", handleClick);
  }, [html]);

  return (
    <div
      ref={containerRef}
      className="prose max-w-none"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
