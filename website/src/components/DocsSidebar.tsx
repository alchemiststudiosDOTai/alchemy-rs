import { Link, useParams } from "react-router-dom";
import { cn } from "../lib/cn";
import type { DocsManifest } from "../lib/types";

interface DocsSidebarProps {
  manifest: DocsManifest;
  open: boolean;
  onClose: () => void;
}

export function DocsSidebar({ manifest, open, onClose }: DocsSidebarProps) {
  const params = useParams();
  const currentSlug = params["*"] || "";

  return (
    <>
      {/* Mobile overlay */}
      {open && (
        <div
          className="fixed inset-0 z-40 md:hidden bg-sidebar"
          onClick={onClose}
        />
      )}

      <aside
        className={cn(
          // Mobile: fixed drawer
          "fixed inset-y-0 left-0 z-50 w-64 bg-sidebar pl-4 border-r border-border transform transition-transform duration-200 ease-in-out md:transform-none",
          // Desktop: static sidebar
          "md:relative md:z-auto md:shrink-0",
          // Mobile open/close
          open ? "translate-x-0" : "-translate-x-full md:translate-x-0"
        )}
      >
        {/* Mobile header with close button */}
        <div className="flex items-center justify-between p-6 border-b border-border md:hidden">
          <span className="font-semibold">Documentation</span>
          <button
            onClick={onClose}
            className="p-1 text-muted-foreground hover:text-foreground"
            aria-label="Close sidebar"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <nav className="sticky top-0 h-[calc(100vh-57px)] overflow-y-auto px-2 py-6">
          <Link
            to="/docs"
            onClick={onClose}
            className={cn(
              "block mb-6 text-sm font-medium transition-colors",
              currentSlug === ""
                ? "text-foreground"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            Overview
          </Link>

          {manifest.categories.map((category) => (
            <div key={category.slug} className="mb-6">
              <h3 className="mb-2 text-sm font-semibold text-foreground">
                {category.name}
              </h3>

              <ul className="space-y-1">
                {category.pages.map((page) => (
                  <li key={page.slug}>
                    <Link
                      to={`/docs/${page.slug}`}
                      onClick={onClose}
                      className={cn(
                        "block py-1 text-sm transition-colors",
                        currentSlug === page.slug
                          ? "font-medium text-foreground"
                          : "text-muted-foreground hover:text-foreground"
                      )}
                    >
                      {page.title}
                    </Link>
                  </li>
                ))}

                {category.subcategories.map((subcategory) => (
                  <li key={subcategory.slug} className="mt-3">
                    <h4 className="mb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                      {subcategory.name}
                    </h4>
                    <ul className="space-y-1 border-l border-border pl-3">
                      {subcategory.pages.map((page) => (
                        <li key={page.slug}>
                          <Link
                            to={`/docs/${page.slug}`}
                            onClick={onClose}
                            className={cn(
                              "block py-1 text-sm transition-colors",
                              currentSlug === page.slug
                                ? "font-medium text-foreground"
                                : "text-muted-foreground hover:text-foreground"
                            )}
                          >
                            {page.title}
                          </Link>
                        </li>
                      ))}
                    </ul>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </nav>
      </aside>
    </>
  );
}
