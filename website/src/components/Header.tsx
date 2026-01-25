import { Link } from "react-router-dom";
import { Logo } from "./Logo";
import { ThemeToggle } from "./ThemeToggle";

export function Header() {
  return (
    <header className="border-b border-border bg-background">
      <div className="container mx-auto border-x px-4 md:px-6 py-4 flex items-center justify-between">
        <Link to="/" className="flex items-center gap-2">
          <Logo width={44} height={25} />
          <span className="text-lg font-bold font-mono">Alchemy</span>
        </Link>
        <nav className="flex items-center gap-4 md:gap-6">
          <Link
            to="/docs"
            className="text-sm text-muted-foreground hover:text-foreground transition-colors hidden sm:block"
          >
            Documentation
          </Link>
          <a
            href="https://github.com/alchemiststudiosDOTai/alchemy-rs"
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-muted-foreground hover:text-foreground transition-colors hidden sm:block"
          >
            GitHub
          </a>
          <ThemeToggle />
        </nav>
      </div>
    </header>
  );
}
