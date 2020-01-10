using Newtonsoft.Json;
using JsonSubTypes;

namespace DotNetGame.Mahjong
{
    [JsonConverter(typeof(JsonSubtypes), "Kind")]
    [JsonSubtypes.KnownSubType(typeof(SimpleTile), TileKind.Simple)]
    [JsonSubtypes.KnownSubType(typeof(HonorTile), TileKind.Honor)]
    [JsonSubtypes.KnownSubType(typeof(BonusTile), TileKind.Bonus)]
    public interface ITile
    {
        TileKind Kind { get; }
    }
}
