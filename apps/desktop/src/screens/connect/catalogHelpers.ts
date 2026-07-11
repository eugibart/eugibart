// Small pure helpers shared by the wizard steps — kept out of the components
// so they're easy to unit-test in isolation from React.
import { CatalogBikeInfo } from "../../ipc";

export function brandsIn(catalog: CatalogBikeInfo[]): string[] {
  return [...new Set(catalog.map((b) => b.brand))].sort();
}

export function modelsFor(catalog: CatalogBikeInfo[], brand: string): CatalogBikeInfo[] {
  return catalog.filter((b) => b.brand === brand);
}

export function yearBounds(entry: CatalogBikeInfo): { min: number; max: number } {
  return { min: entry.year_from, max: entry.year_to ?? new Date().getFullYear() };
}
