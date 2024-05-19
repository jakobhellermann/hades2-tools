declare module "@pgrabovets/json-view" {
  class Tree {}

  declare function create(data: string | unknown): Tree;
  declare function render(tree: Tree, element: HTMLElement);
  declare function renderJSON(data: string | unknown, element: HTMLElement);
}
