"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const NAV_SECTIONS = [
  {
    label: "OVERVIEW",
    links: [
      { href: "/", label: "Dashboard", icon: "◈" },
    ],
  },
  {
    label: "INFRASTRUCTURE",
    links: [
      { href: "/admin/vms", label: "Virtual Machines", icon: "▣" },
      { href: "/admin/networks", label: "Networks", icon: "⬡" },
      { href: "/admin/storage", label: "Storage", icon: "⬢" },
    ],
  },
  {
    label: "SYSTEM",
    links: [
      { href: "/console", label: "Console", icon: ">" },
    ],
  },
];

export function Sidebar() {
  const pathname = usePathname();

  function isActive(href: string): boolean {
    if (href === "/") return pathname === "/";
    return pathname === href || pathname.startsWith(href + "/");
  }

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <div className="brand-icon">T</div>
        THISCLOUD
      </div>

      {NAV_SECTIONS.map((section) => (
        <div className="sidebar-section" key={section.label}>
          <div className="sidebar-section-label">{section.label}</div>
          <nav className="sidebar-nav">
            {section.links.map((link) => (
              <Link
                key={link.href}
                href={link.href}
                className={`sidebar-link ${isActive(link.href) ? "active" : ""}`}
              >
                <span className="link-icon">{link.icon}</span>
                {link.label}
              </Link>
            ))}
          </nav>
        </div>
      ))}

      <div className="sidebar-footer">
        THISCLOUD v0.1
      </div>
    </aside>
  );
}
