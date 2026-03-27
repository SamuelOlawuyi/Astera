'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';

const adminLinks = [
  {
    href: '/admin/dashboard',
    label: 'Dashboard',
    icon: 'M3 13h8V3H3v10zm0 8h8v-6H3v6zm10 0h8V11h-8v10zm0-18v6h8V3h-8z',
  },
  {
    href: '/admin/invoices',
    label: 'Invoices',
    icon: 'M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8l-6-6zM6 4h7v5h5v11H6V4z',
  },
  {
    href: '/admin/yield',
    label: 'Yield Rate',
    icon: 'M13 7h-2v4H7v2h4v4h2v-4h4v-2h-4V7zm-1-5C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z',
  },
  {
    href: '/admin/defaults',
    label: 'Defaults',
    icon: 'M1 21h22L12 2 1 21zm12-3h-2v-2h2v2zm0-4h-2v-4h2v4z',
  },
];

export default function AdminNav() {
  const path = usePathname();

  return (
    <nav className="w-64 bg-brand-card border-r border-brand-border h-[calc(100vh-64px)] fixed left-0 top-16 hidden lg:flex flex-col p-4 gap-2">
      <div className="mb-6 px-4">
        <h2 className="text-xs font-bold uppercase tracking-widest text-brand-muted">
          Admin Panel
        </h2>
      </div>

      {adminLinks.map((link) => {
        const isActive =
          path === link.href || (link.href === '/admin/dashboard' && path === '/admin');

        return (
          <Link
            key={link.href}
            href={link.href}
            className={`flex items-center gap-3 px-4 py-3 rounded-xl text-sm font-medium transition-all duration-200 ${
              isActive
                ? 'bg-brand-gold/10 text-brand-gold shadow-inner'
                : 'text-brand-muted hover:text-white hover:bg-brand-dark/50'
            }`}
          >
            <svg
              className={`w-5 h-5 ${isActive ? 'text-brand-gold' : 'text-brand-muted'}`}
              fill="currentColor"
              viewBox="0 0 24 24"
            >
              <path d={link.icon} />
            </svg>
            {link.label}
          </Link>
        );
      })}
    </nav>
  );
}
