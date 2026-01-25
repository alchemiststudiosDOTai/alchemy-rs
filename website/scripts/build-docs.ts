import fs from "fs";
import path from "path";
import matter from "gray-matter";

interface DocPage {
  slug: string;
  title: string;
  summary: string;
  readWhen: string[];
  content: string;
  category: string;
  subcategory?: string;
  order: number;
}

interface DocCategory {
  name: string;
  slug: string;
  pages: DocPage[];
  subcategories: {
    name: string;
    slug: string;
    pages: DocPage[];
  }[];
}

interface DocsManifest {
  categories: DocCategory[];
  pages: Record<string, DocPage>;
}

const DOCS_DIR = path.resolve(__dirname, "../../docs");
const OUTPUT_DIR = path.resolve(__dirname, "../public/docs");

function slugify(str: string): string {
  return str
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
}

function extractTitle(content: string, filename: string): string {
  const match = content.match(/^#\s+(.+)$/m);
  if (match) {
    return match[1].trim();
  }
  return filename
    .replace(/\.md$/, "")
    .split("-")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

function getOrderFromFilename(filename: string): number {
  const match = filename.match(/^(\d+)-/);
  if (match) {
    return parseInt(match[1], 10);
  }
  return 999;
}

function processMarkdownFile(
  filePath: string,
  relativePath: string
): DocPage | null {
  try {
    const fileContent = fs.readFileSync(filePath, "utf-8");
    const { data: frontmatter, content } = matter(fileContent);

    const pathParts = relativePath.split(path.sep);
    const filename = pathParts[pathParts.length - 1];
    const category = pathParts[0] || "general";
    const subcategory = pathParts.length > 2 ? pathParts[1] : undefined;

    const slug = relativePath.replace(/\.md$/, "").replace(/\\/g, "/");
    const title = extractTitle(content, filename);
    const order = getOrderFromFilename(filename);

    return {
      slug,
      title,
      summary: frontmatter.summary || "",
      readWhen: frontmatter.read_when || [],
      content,
      category,
      subcategory,
      order,
    };
  } catch (error) {
    console.error(`Error processing ${filePath}:`, error);
    return null;
  }
}

function walkDirectory(dir: string, baseDir: string = dir): string[] {
  const files: string[] = [];

  if (!fs.existsSync(dir)) {
    return files;
  }

  const entries = fs.readdirSync(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...walkDirectory(fullPath, baseDir));
    } else if (entry.isFile() && entry.name.endsWith(".md")) {
      files.push(fullPath);
    }
  }

  return files;
}

function buildDocsManifest(): DocsManifest {
  const markdownFiles = walkDirectory(DOCS_DIR);
  const pages: Record<string, DocPage> = {};
  const categoryMap = new Map<string, DocCategory>();

  for (const filePath of markdownFiles) {
    const relativePath = path.relative(DOCS_DIR, filePath);
    const page = processMarkdownFile(filePath, relativePath);

    if (!page) continue;

    pages[page.slug] = page;

    if (!categoryMap.has(page.category)) {
      categoryMap.set(page.category, {
        name: page.category
          .split("-")
          .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
          .join(" "),
        slug: slugify(page.category),
        pages: [],
        subcategories: [],
      });
    }

    const category = categoryMap.get(page.category)!;

    if (page.subcategory) {
      let subcategory = category.subcategories.find(
        (s) => s.slug === slugify(page.subcategory!)
      );
      if (!subcategory) {
        subcategory = {
          name: page.subcategory
            .split("-")
            .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
            .join(" "),
          slug: slugify(page.subcategory),
          pages: [],
        };
        category.subcategories.push(subcategory);
      }
      subcategory.pages.push(page);
    } else {
      category.pages.push(page);
    }
  }

  // Sort pages within each category by order
  for (const category of categoryMap.values()) {
    category.pages.sort((a, b) => a.order - b.order);
    for (const subcategory of category.subcategories) {
      subcategory.pages.sort((a, b) => a.order - b.order);
    }
    category.subcategories.sort((a, b) => a.slug.localeCompare(b.slug));
  }

  const categories = Array.from(categoryMap.values()).sort((a, b) =>
    a.slug.localeCompare(b.slug)
  );

  return { categories, pages };
}

function main() {
  console.log("Building docs manifest...");
  console.log(`Reading from: ${DOCS_DIR}`);
  console.log(`Writing to: ${OUTPUT_DIR}`);

  if (!fs.existsSync(DOCS_DIR)) {
    console.error(`Docs directory not found: ${DOCS_DIR}`);
    process.exit(1);
  }

  fs.mkdirSync(OUTPUT_DIR, { recursive: true });

  const manifest = buildDocsManifest();

  // Write the manifest
  fs.writeFileSync(
    path.join(OUTPUT_DIR, "manifest.json"),
    JSON.stringify(manifest, null, 2)
  );

  // Write individual page files for lazy loading
  for (const [slug, page] of Object.entries(manifest.pages)) {
    const pageDir = path.join(OUTPUT_DIR, "pages", path.dirname(slug));
    fs.mkdirSync(pageDir, { recursive: true });
    fs.writeFileSync(
      path.join(OUTPUT_DIR, "pages", `${slug}.json`),
      JSON.stringify(page, null, 2)
    );
  }

  console.log(`Built ${Object.keys(manifest.pages).length} doc pages`);
  console.log(
    `Categories: ${manifest.categories.map((c) => c.name).join(", ")}`
  );
}

main();
