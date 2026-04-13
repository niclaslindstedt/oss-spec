export default function Footer() {
  return (
    <footer className="border-t border-border py-12">
      <div className="mx-auto max-w-6xl px-6">
        <div className="flex flex-col items-center justify-between gap-6 md:flex-row">
          <div>
            <span className="text-lg font-bold text-text-primary">
              <span className="text-accent">&#x1f6e1;&#xfe0f;</span> oss-spec
            </span>
            <p className="mt-1 text-sm text-text-dim">Bootstrap spec-compliant open source repos</p>
          </div>

          <div className="flex flex-wrap justify-center gap-x-6 gap-y-2 text-sm text-text-secondary">
            <a
              href="https://github.com/niclaslindstedt/oss-spec"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              GitHub
            </a>
            <a
              href="https://crates.io/crates/oss-spec"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              crates.io
            </a>
            <a
              href="https://github.com/niclaslindstedt/oss-spec/blob/main/OSS_SPEC.md"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              OSS_SPEC.md
            </a>
            <a
              href="https://github.com/niclaslindstedt/oss-spec/blob/main/LICENSE"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              MIT License
            </a>
          </div>
        </div>
      </div>
    </footer>
  );
}
