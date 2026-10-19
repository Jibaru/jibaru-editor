import React from "react";
import { interpolate } from "remotion";
import { C } from "../theme";

/**
 * Jibaru Editor mark: a rounded tile with a bold "J" whose stroke doubles as
 * a timeline clip, and a red playhead dot. `p` (0..1) draws the J in.
 */
export const LogoMark: React.FC<{ size: number; p?: number }> = ({ size, p = 1 }) => {
  const draw = interpolate(p, [0, 1], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  return (
    <svg width={size} height={size} viewBox="0 0 100 100">
      <defs>
        <linearGradient id="jb-tile" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#52e8c4" />
          <stop offset="1" stopColor="#178a72" />
        </linearGradient>
        <linearGradient id="jb-shine" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#ffffff" stopOpacity="0.28" />
          <stop offset="0.5" stopColor="#ffffff" stopOpacity="0" />
        </linearGradient>
      </defs>
      <rect x="2" y="2" width="96" height="96" rx="26" fill="url(#jb-tile)" />
      <rect x="2" y="2" width="96" height="96" rx="26" fill="url(#jb-shine)" />
      <path
        d="M 62 24 L 62 58 A 17 17 0 0 1 28 58"
        fill="none"
        stroke={C.bg}
        strokeWidth="13"
        strokeLinecap="round"
        strokeLinejoin="round"
        pathLength={1}
        strokeDasharray={1}
        strokeDashoffset={draw}
      />
      <circle cx="35" cy="28" r={7.5 * Math.min(1, p * 1.5)} fill={C.playhead} />
    </svg>
  );
};
