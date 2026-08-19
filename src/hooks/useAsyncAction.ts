import { useCallback, useState } from "react";

type AsyncSuccess<T> = {
  ok: true;
  value: T;
};

type AsyncFailure = {
  ok: false;
  error: string;
};

export type AsyncResult<T> = AsyncSuccess<T> | AsyncFailure;

function messageFrom(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Something went wrong. Please try again.";
}

export function useAsyncAction() {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(async <T,>(work: () => Promise<T>): Promise<AsyncResult<T>> => {
    setPending(true);
    setError(null);

    try {
      const value = await work();
      return { ok: true, value };
    } catch (caughtError) {
      const nextError = messageFrom(caughtError);
      setError(nextError);
      return { ok: false, error: nextError };
    } finally {
      setPending(false);
    }
  }, []);

  return {
    pending,
    error,
    clearError: () => setError(null),
    run,
  };
}
