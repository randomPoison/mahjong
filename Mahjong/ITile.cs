using Newtonsoft.Json;
using JsonSubTypes;

namespace DotNetGame.Mahjong
{
    [JsonConverter(typeof(JsonSubtypes), "Kind")]
    public interface ITile
    {
        TileKind Kind { get; }
    }
}
