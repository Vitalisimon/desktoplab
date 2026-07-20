import type { AppRoute } from "./routes";

type RouteBoundaryProps = {
  route: AppRoute;
};

export function RouteBoundary({ route }: RouteBoundaryProps) {
  return <div data-route-boundary={route} data-testid="route-boundary" />;
}
