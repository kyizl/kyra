export interface AutoFitWindowOptions {
  currentWidth: number;
  desiredContentWidth: number;
  lastRequestedContentWidth: number;
  minGrowDelta?: number;
  minContentDelta?: number;
  maxGrowDelta?: number;
}

export function shouldAutoFitWindow({
  currentWidth,
  desiredContentWidth,
  lastRequestedContentWidth,
  minGrowDelta = 24,
  minContentDelta = 12,
  maxGrowDelta = 480,
}: AutoFitWindowOptions): boolean {
  if (desiredContentWidth <= 0) return false;

  const widthDelta = desiredContentWidth - currentWidth;
  if (widthDelta < minGrowDelta) return false;
  if (widthDelta > maxGrowDelta) return false;

  if (lastRequestedContentWidth > 0) {
    if (Math.abs(desiredContentWidth - lastRequestedContentWidth) < minContentDelta) {
      return false;
    }
  }

  return true;
}
