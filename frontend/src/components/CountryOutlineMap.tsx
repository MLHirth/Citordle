import world from "@svg-maps/world";
import { svgPathBbox } from "svg-path-bbox";

type CountryOutlineMapProps = {
  countryCode?: string;
  countryName: string;
  fallbackPath?: string;
};

const COUNTRY_ALIASES: Record<string, string[]> = {
  "united states": ["United States of America", "United States"],
  "united kingdom": ["United Kingdom"]
};

type WorldLocation = {
  id: string;
  name: string;
  path: string;
};

const worldLocations = (world as { locations: WorldLocation[] }).locations;

export default function CountryOutlineMap({
  countryCode,
  countryName,
  fallbackPath
}: CountryOutlineMapProps) {
  const normalizedCode = countryCode?.trim().toLowerCase();
  const normalizedName = countryName.trim().toLowerCase();

  const byCode = normalizedCode
    ? worldLocations.find((location) => location.id.toLowerCase() === normalizedCode)
    : undefined;

  const byName = worldLocations.find((location) => {
    const name = location.name.toLowerCase();
    if (name === normalizedName) {
      return true;
    }
    const aliases = COUNTRY_ALIASES[normalizedName] ?? [];
    return aliases.some((alias) => alias.toLowerCase() === name);
  });

  const path = byCode?.path ?? byName?.path ?? fallbackPath;

  if (!path) {
    return <p>Map outline unavailable.</p>;
  }

  let viewBox = "0 0 200 120";
  try {
    const [minX, minY, maxX, maxY] = svgPathBbox(path);
    const width = maxX - minX;
    const height = maxY - minY;
    const pad = Math.max(width, height) * 0.12;
    viewBox = `${minX - pad} ${minY - pad} ${width + pad * 2} ${height + pad * 2}`;
  } catch {
    viewBox = "0 0 200 120";
  }

  return (
    <svg viewBox={viewBox} role="img" aria-label={`${countryName} outline`}>
      <path className="map-country" d={path} />
    </svg>
  );
}
