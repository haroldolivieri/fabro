import type { Route } from "./+types/workflows";

export function meta({}: Route.MetaArgs) {
  return [{ title: "Workflows — Arc" }];
}

export default function Workflows() {
  return null;
}
