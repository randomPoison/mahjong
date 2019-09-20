using UniRx.Async;
using UnityEngine.ResourceManagement;

public static class AsyncUtils
{
    public static UniTask<T>.Awaiter GetAwaiter<T>(this IAsyncOperation<T> asyncOperation)
    {
        var completion = new UniTaskCompletionSource<T>();
        asyncOperation.Completed += (operation) =>
        {
            completion.TrySetResult(operation.Result);
        };
        return completion.Task.GetAwaiter();
    }
}
