"use client";

import { Header } from "@/components/header";

export default function AdminLoading() {
  return (
    <>
      <Header title="Admin" />
      <main className="content">
        <div className="page-header">
          <div>
            <h1 className="page-title">Virtual Machines</h1>
            <p className="page-subtitle">Loading resources...</p>
          </div>
        </div>
        <div className="loading-page">
          <div className="spinner" />
          <span>Loading admin panel...</span>
        </div>
      </main>
    </>
  );
}
