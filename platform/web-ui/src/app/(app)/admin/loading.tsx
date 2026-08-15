"use client";

export default function AdminLoading() {
  return (
    <div className="content">
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
    </div>
  );
}
