import type { JSX } from "solid-js";

// Lucide-style line icons (ISC-licensed shapes), 1.75px stroke, decorative.
const P: Record<string, () => JSX.Element> = {
  pointer: () => <path d="M3 3l7.07 16.97 2.51-7.39 7.39-2.51L3 3z" />,
  ruler: () => <><path d="M21.3 15.3a2.4 2.4 0 0 1 0 3.4l-2.6 2.6a2.4 2.4 0 0 1-3.4 0L2.7 8.7a2.41 2.41 0 0 1 0-3.4l2.6-2.6a2.41 2.41 0 0 1 3.4 0Z" /><path d="m14.5 12.5 2-2" /><path d="m11.5 9.5 2-2" /><path d="m8.5 6.5 2-2" /><path d="m17.5 15.5 2-2" /></>,
  ping: () => <><circle cx="12" cy="12" r="3" /><circle cx="12" cy="12" r="8" /></>,
  grid: () => <><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M3 9h18M3 15h18M9 3v18M15 3v18" /></>,
  plus: () => <path d="M5 12h14M12 5v14" />,
  minus: () => <path d="M5 12h14" />,
  center: () => <><path d="M12 2v4M12 18v4M2 12h4M18 12h4" /><circle cx="12" cy="12" r="4" /></>,
  book: () => <><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z" /><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z" /></>,
  journal: () => <path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20" />,
  chat: () => <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />,
  search: () => <><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></>,
  sliders: () => <><line x1="21" x2="14" y1="4" y2="4" /><line x1="10" x2="3" y1="4" y2="4" /><line x1="21" x2="12" y1="12" y2="12" /><line x1="8" x2="3" y1="12" y2="12" /><line x1="21" x2="16" y1="20" y2="20" /><line x1="12" x2="3" y1="20" y2="20" /><line x1="14" x2="14" y1="2" y2="6" /><line x1="8" x2="8" y1="10" y2="14" /><line x1="16" x2="16" y1="18" y2="22" /></>,
  collapse: () => <><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M15 3v18" /><path d="m8 9 3 3-3 3" /></>,
  expand: () => <><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M15 3v18" /><path d="m11 9-3 3 3 3" /></>,
  chevron: () => <path d="m6 9 6 6 6-6" />,
  back: () => <path d="m15 18-6-6 6-6" />,
  close: () => <path d="M18 6 6 18M6 6l12 12" />,
  dice: () => <><rect x="3" y="3" width="18" height="18" rx="3" /><circle cx="8" cy="8" r="1" /><circle cx="16" cy="16" r="1" /><circle cx="12" cy="12" r="1" /></>,
  trash: () => <><path d="M3 6h18" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" /><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" /></>,
  image: () => <><rect x="3" y="3" width="18" height="18" rx="2" /><circle cx="9" cy="9" r="2" /><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21" /></>,
  map: () => <><path d="M9 4 3 6v14l6-2 6 2 6-2V4l-6 2-6-2z" /><path d="M9 4v14M15 6v14" /></>,
};

export function Icon(props: { name: keyof typeof P | string; size?: number }) {
  const s = () => props.size ?? 18;
  return (
    <svg width={s()} height={s()} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      {P[props.name]?.()}
    </svg>
  );
}

export function StopIcon() {
  return <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><rect x="6" y="6" width="12" height="12" rx="1" /></svg>;
}
