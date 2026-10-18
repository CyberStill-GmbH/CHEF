interface ErrorStateProps {
  message: string;
  onRetry: () => void;
}

export function ErrorState({ message, onRetry }: ErrorStateProps) {
  return (
    <section role="alert">
      <h2>No se pudo mostrar el snapshot</h2>
      <p>{message}</p>
      <button type="button" onClick={onRetry}>
        Reintentar
      </button>
    </section>
  );
}
