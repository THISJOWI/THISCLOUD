import { Sidebar } from "@/components/sidebar";
import { Header } from "@/components/header";

export default function AppLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="shell">
      <Header />
      <div className="shell-body">
        <Sidebar />
        <main className="main-area">{children}</main>
      </div>
    </div>
  );
}