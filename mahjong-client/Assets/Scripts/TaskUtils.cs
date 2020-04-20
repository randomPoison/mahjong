using System;
using System.Threading;
using System.Threading.Tasks;

public static class TaskUtils
{
    /// <summary>
    /// Adds a cancellation token to a task after its creation.
    /// </summary>
    ///
    /// <param name="task">The task to append the cancellation token to.</param>
    /// <param name="cancellationToken">The cancellation token to append.</param>
    ///
    /// <returns>
    /// A new task that will either resolve to the value resolved by <paramref name="task"/>,
    /// or will be canceled when <paramref name="cancellationToken"/> fires.
    /// </returns>
    public static async Task<T> WithCancellation<T>(this Task<T> task, CancellationToken cancellationToken)
    {
        var tcs = new TaskCompletionSource<bool>();
        using (cancellationToken.Register(s => ((TaskCompletionSource<bool>)s).TrySetResult(true), tcs))
        {
            if (task != await Task.WhenAny(task, tcs.Task))
            {
                throw new OperationCanceledException(cancellationToken);
            }
        }
        return await task;
    }
}
