declare module "@pgrabovets/json-view" {
  class Tree {}

  declare function create(data: string | unknown): Tree;
  declare function render(tree: Tree, element: HTMLElement);
  declare function renderJSON(data: string | unknown, element: HTMLElement);
  declare function expand(tree: Tree);
  declare function collapse(tree: Tree);
  declare function traverse(tree: Tree, f: (node: unknown) => void);
  declare function toggleNode(tree: Tree);
  declare function destroy(tree: Tree);
}
