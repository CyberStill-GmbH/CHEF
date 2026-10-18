import type { AssetDto } from '../contracts/snapshot';

interface AssetListProps {
  assets: AssetDto[];
  selectedAssetId: string | null;
  onSelect: (assetId: string) => void;
}

export function AssetList({
  assets,
  selectedAssetId,
  onSelect,
}: AssetListProps) {
  return (
    <section aria-labelledby="asset-list-title">
      <h2 id="asset-list-title">Assets disponibles</h2>
      <ul>
        {assets.map((asset) => (
          <li key={asset.id}>
            <button
              type="button"
              aria-pressed={asset.id === selectedAssetId}
              onClick={() => onSelect(asset.id)}
            >
              <span>
                Kind: <span>{asset.kind}</span>
              </span>{' '}
              <span>
                Key: <span>{asset.key}</span>
              </span>{' '}
              <span>
                First seen: <span>{asset.firstSeen}</span>
              </span>{' '}
              <span>
                Last seen: <span>{asset.lastSeen}</span>
              </span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
