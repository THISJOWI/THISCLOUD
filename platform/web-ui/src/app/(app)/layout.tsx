import { Sidebar } from "@/components/sidebar";

export default function AppLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="layout">
      <Sidebar />
      <div className="main-area">{children}</div>
    </div>
  );
}
