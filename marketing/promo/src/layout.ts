import { useVideoConfig } from "remotion";

/**
 * Layout helper shared by every scene. `u` is a unit that scales with the
 * short side of the frame (1 u = 1 px at 1080), so the same scene code works
 * for 1080x1920 and 1920x1080.
 *
 * Vertical safe area: platform UI covers ~200 px at the top and ~350 px at
 * the bottom, so text stays between `safeTop` and `H - safeBottom`.
 */
export const useLayout = () => {
  const { width: W, height: H } = useVideoConfig();
  const vertical = H > W;
  const square = H === W;
  const u = Math.min(W, H) / 1080;
  const safeTop = vertical ? 220 : 70 * u;
  const safeBottom = vertical ? 380 : 70 * u;
  return { W, H, vertical, square, u, safeTop, safeBottom, safeH: H - safeTop - safeBottom };
};
