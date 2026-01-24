import { useEffect, useState, useMemo } from "react";
import { useParams, Link } from "react-router-dom";
import { MarkdownContent } from "../components/MarkdownContent";
import type { DocPage } from "../lib/types";

function formatCategoryName(slug: string): string {
  return slug
    .split("-")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

function removeFirstHeading(content: string): string {
  // Remove the first H1 heading from markdown content (handles leading whitespace and CRLF)
  return content.replace(/^\s*#\s+.+[\r\n]+/, "");
}

interface TocItem {
  id: string;
  text: string;
  level: number;
}

function extractToc(content: string): TocItem[] {
  const headingRegex = /^(#{2,3})\s+(.+)$/gm;
  const items: TocItem[] = [];
  let match;

  while ((match = headingRegex.exec(content)) !== null) {
    const level = match[1].length;
    const text = match[2].trim();
    const id = text
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/(^-|-$)/g, "");
    items.push({ id, text, level });
  }

  return items;
}

export function DocsPage() {
  const params = useParams();
  const slug = params["*"] || "";

  const [page, setPage] = useState<DocPage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    setError(null);
    window.scrollTo(0, 0);

    fetch(`/docs/pages/${slug}.json`)
      .then((res) => {
        if (!res.ok) throw new Error("Page not found");
        return res.json();
      })
      .then((data) => {
        setPage(data);
        setLoading(false);
      })
      .catch((err) => {
        setError(err.message);
        setLoading(false);
      });
  }, [slug]);

  const toc = useMemo(() => {
    if (!page) return [];
    return extractToc(page.content);
  }, [page]);

  const contentWithoutTitle = useMemo(() => {
    if (!page) return "";
    return removeFirstHeading(page.content);
  }, [page]);

  if (loading) {
    return (
      <div className="py-8">
        <p className="text-muted-foreground">Loading...</p>
      </div>
    );
  }

  if (error || !page) {
    return (
      <div className="py-8">
        <h1 className="text-2xl font-bold mb-2">Page not found</h1>
        <p className="text-muted-foreground mb-4">
          The documentation page you're looking for doesn't exist.
        </p>
        <Link to="/docs" className="text-sm font-medium hover:underline">
          Back to documentation
        </Link>
      </div>
    );
  }

  return (
    <div className="flex gap-8">
      <article className="flex-1 min-w-0">
        <div className="mb-8">
          {/* Breadcrumb navigation */}
          <nav className="flex items-center gap-2 text-sm mb-4">
            <Link
              to="/docs"
              className="text-muted-foreground hover:text-foreground transition-colors"
            >
              Docs
            </Link>
            <span className="text-muted-foreground">/</span>
            <span className="text-muted-foreground">
              {formatCategoryName(page.category)}
            </span>
            {page.subcategory && (
              <>
                <span className="text-muted-foreground">/</span>
                <span className="text-muted-foreground">
                  {formatCategoryName(page.subcategory)}
                </span>
              </>
            )}
            <span className="text-muted-foreground">/</span>
            <span className="text-foreground font-medium">{page.title}</span>
          </nav>

          {/* Page title */}
          <h1 className="text-3xl font-bold mb-4">{page.title}</h1>

          {/* Description */}
          {page.summary && (
            <p className="text-muted-foreground mb-4">{page.summary}</p>
          )}

          {/* Read this when box */}
          {page.readWhen && page.readWhen.length > 0 && (
            <div className="p-4 bg-muted/50 border border-border">
              <h4 className="text-sm font-medium mb-2">Read this when:</h4>
              <ul className="text-sm text-muted-foreground space-y-1">
                {page.readWhen.map((item, i) => (
                  <li key={i}>• {item}</li>
                ))}
              </ul>
            </div>
          )}
        </div>

        <MarkdownContent content={contentWithoutTitle} />
      </article>

      {/* On this page sidebar */}
      {toc.length > 0 && (
        <aside className="hidden lg:block w-64 shrink-0">
          <div className="sticky top-6 ml-auto w-48">
            <h4 className="text-sm font-medium mb-3">On this page</h4>
            <nav className="space-y-2">
              {toc.map((item) => (
                <a
                  key={item.id}
                  href={`#${item.id}`}
                  className={`block text-sm text-muted-foreground hover:text-foreground transition-colors ${
                    item.level === 3 ? "pl-3" : ""
                  }`}
                >
                  {item.text}
                </a>
              ))}
            </nav>
          </div>
        </aside>
      )}
    </div>
  );
}
