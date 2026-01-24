import { Link, useOutletContext } from "react-router-dom";
import type { DocsManifest } from "../lib/types";

export function DocsIndex() {
  const { manifest } = useOutletContext<{ manifest: DocsManifest }>();

  return (
    <div className="pr-12">
      <h1 className="text-3xl font-bold mb-2">Documentation</h1>
      <p className="text-muted-foreground mb-8">
        Learn how to use Alchemy, the unified LLM API abstraction layer in Rust.
      </p>

      <div className="space-y-8">
        {manifest.categories.map((category) => (
          <section key={category.slug}>
            <h2 className="text-xl font-semibold mb-4 pb-2 border-b border-border">
              {category.name}
            </h2>

            <div className="grid gap-4">
              {category.pages.map((page) => (
                <Link
                  key={page.slug}
                  to={`/docs/${page.slug}`}
                  className="block p-4 border border-border hover:border-foreground/20 transition-colors"
                >
                  <h3 className="font-medium mb-1">{page.title}</h3>
                  {page.summary && (
                    <p className="text-sm text-muted-foreground">
                      {page.summary}
                    </p>
                  )}
                </Link>
              ))}

              {category.subcategories.map((subcategory) => (
                <div key={subcategory.slug} className="mt-2">
                  <h3 className="text-sm font-medium uppercase tracking-wide text-muted-foreground mb-3">
                    {subcategory.name}
                  </h3>
                  <div className="grid gap-4 pl-4 border-l border-border">
                    {subcategory.pages.map((page) => (
                      <Link
                        key={page.slug}
                        to={`/docs/${page.slug}`}
                        className="block p-4 border border-border hover:border-foreground/20 transition-colors"
                      >
                        <h4 className="font-medium mb-1">{page.title}</h4>
                        {page.summary && (
                          <p className="text-sm text-muted-foreground">
                            {page.summary}
                          </p>
                        )}
                      </Link>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}
