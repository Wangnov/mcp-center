import type { SVGProps } from "react";

export function McpCenterLogo(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 200 200"
      fill="none"
      {...props}
    >
      {/* 轨道系统设计 - MCP Center Logo */}

      {/* 中心核心 */}
      <circle cx="100" cy="100" r="16" fill="currentColor" />

      {/* 内层轨道 */}
      <circle
        cx="100"
        cy="100"
        r="45"
        stroke="currentColor"
        strokeWidth="2"
        opacity="0.3"
      />

      {/* 中层轨道 */}
      <circle
        cx="100"
        cy="100"
        r="70"
        stroke="currentColor"
        strokeWidth="2"
        opacity="0.2"
      />

      {/* 外层轨道 */}
      <circle
        cx="100"
        cy="100"
        r="95"
        stroke="currentColor"
        strokeWidth="2"
        opacity="0.2"
      />

      {/* 内层节点（3个） */}
      <circle cx="100" cy="55" r="8" fill="currentColor" />
      <circle cx="139" cy="122.5" r="8" fill="currentColor" />
      <circle cx="61" cy="122.5" r="8" fill="currentColor" />

      {/* 中层节点（4个） */}
      <circle cx="100" cy="30" r="6" fill="currentColor" opacity="0.6" />
      <circle cx="170" cy="100" r="6" fill="currentColor" opacity="0.6" />
      <circle cx="100" cy="170" r="6" fill="currentColor" opacity="0.6" />
      <circle cx="30" cy="100" r="6" fill="currentColor" opacity="0.6" />

      {/* 连接线（从节点到中心的能量流） */}
      <line
        x1="100"
        y1="55"
        x2="100"
        y2="100"
        stroke="currentColor"
        strokeWidth="2"
        opacity="0.5"
      />
      <line
        x1="139"
        y1="122.5"
        x2="100"
        y2="100"
        stroke="currentColor"
        strokeWidth="2"
        opacity="0.5"
      />
      <line
        x1="61"
        y1="122.5"
        x2="100"
        y2="100"
        stroke="currentColor"
        strokeWidth="2"
        opacity="0.5"
      />
    </svg>
  );
}
