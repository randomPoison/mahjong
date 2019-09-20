using Newtonsoft.Json;
using UniRx.Async;
using UnityEngine;
using WebSocketSharp;

public static class WebSocketUtils
{
    public static async UniTask<T> RecvMessageAsync<T>(this WebSocket socket)
    {
        var messageString = await socket.RecvStringAsync();
        Debug.LogFormat("Got message string: {0}", messageString);
        return JsonConvert.DeserializeObject<T>(messageString);
    }
}
