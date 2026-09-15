export interface NavItem {
  href: string;
  label: string;
}

/** Primary navigation for §11.1 screens. Order matches the clinical workflow. */
export const navItems: NavItem[] = [
  { href: "/config", label: "Config builder" },
  { href: "/preview", label: "Structure preview" },
  { href: "/generate", label: "Generate" },
  { href: "/package", label: "Package viewer" },
  { href: "/unblinded", label: "Unblinded view" },
  { href: "/validation", label: "Validation" },
  { href: "/about", label: "About" },
];
