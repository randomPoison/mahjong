using System.Collections.Generic;
using Newtonsoft.Json;
using Newtonsoft.Json.Converters;
using Newtonsoft.Json.Linq;
using UnityEngine;

public class GameStateData
{
    // TODO: Create a strong type for player IDs.
    [JsonProperty("players")]
    public Dictionary<int, PlayerData> Players { get; private set; }
}

public class PlayerData
{
    [JsonProperty("pos")]
    public GridPos Pos { get; private set; }

    [JsonProperty("health")]
    public HealthData Health { get; private set; }

    [JsonProperty("pending_turn")]
    public TurnData PendingTurn { get; private set; }
}

public class HealthData
{
    [JsonProperty("max")]
    public int Max { get; private set; }

    [JsonProperty("current")]
    public int Current { get; private set; }
}

public class TurnData
{
    [JsonProperty("movement")]
    public GridPos? Movement { get; private set; }
}

public struct GridPos
{
    public int x;

    public int y;

    public Vector3Int WorldPos
    {
        get { return new Vector3Int(x, 0, y); }
    }
}

public struct Message
{
    [JsonProperty("type")]
    public readonly MessageType Type;

    [JsonProperty("data")]
    public readonly JObject Data;
}

[JsonConverter(typeof(StringEnumConverter))]
public enum MessageType
{
    PlayerAdded,
    SetMovement,
}

public struct PlayerAdded
{
    [JsonProperty("id")]
    public readonly int Id;

    [JsonProperty("data")]
    public readonly PlayerData Data;
}

public struct SetMovement
{
    [JsonProperty("id")]
    public readonly int Id;

    [JsonProperty("pos")]
    public readonly GridPos Pos;
}
