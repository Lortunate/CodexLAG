import type { RuntimeLogFileMetadata } from "../../lib/types";

interface RuntimeLogFilesTableProps {
  files: RuntimeLogFileMetadata[];
}

function formatTimestamp(epochMillis: number) {
  const date = new Date(epochMillis);
  return Number.isNaN(date.getTime()) ? "unknown" : date.toISOString();
}

export function RuntimeLogFilesTable({ files }: RuntimeLogFilesTableProps) {
  if (files.length === 0) {
    return <p>No runtime log files available.</p>;
  }

  return (
    <table aria-label="Runtime log files metadata">
      <thead>
        <tr>
          <th scope="col">Name</th>
          <th scope="col">Path</th>
          <th scope="col">Size (bytes)</th>
          <th scope="col">Modified</th>
        </tr>
      </thead>
      <tbody>
        {files.map((file) => (
          <tr key={file.path}>
            <td>{file.name}</td>
            <td>{file.path}</td>
            <td>{file.size}</td>
            <td>{formatTimestamp(file.mtime)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
