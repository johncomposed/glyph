import type { FontBakeDescriptor, FontVariationRequest } from '../font-baker/index.js';

interface CoreFontArtifact {
  readonly role: 'font';
}

export function fontBakeDescriptor(fontFaceIndex: number, variation?: FontVariationRequest): FontBakeDescriptor {
  return { formatVersion: 0, fontFaceIndex, ...(variation === undefined ? {} : { variation }) };
}

export function soleCoreFontArtifact<Artifact extends CoreFontArtifact>(result: {
  readonly artifacts: readonly Artifact[];
}): Artifact {
  const artifact = result.artifacts[0];
  if (result.artifacts.length !== 1 || artifact?.role !== 'font') {
    throw new TypeError('font bake result must contain exactly one core font artifact');
  }
  return artifact;
}
