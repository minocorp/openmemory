export function pageTitle(name?: string) {
  return { meta: [{ title: name ? `${name} - OpenMemory` : "OpenMemory" }] };
}
