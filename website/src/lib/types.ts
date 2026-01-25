export interface DocPage {
  slug: string;
  title: string;
  summary: string;
  readWhen: string[];
  content: string;
  category: string;
  subcategory?: string;
  order: number;
}

export interface DocSubcategory {
  name: string;
  slug: string;
  pages: DocPage[];
}

export interface DocCategory {
  name: string;
  slug: string;
  pages: DocPage[];
  subcategories: DocSubcategory[];
}

export interface DocsManifest {
  categories: DocCategory[];
  pages: Record<string, DocPage>;
}
