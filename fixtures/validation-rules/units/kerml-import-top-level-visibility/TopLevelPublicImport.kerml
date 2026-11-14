package TopLevelImports;

// Fires: a public import owned by the root namespace.
public import TopLevelImports::*;

// Does not fire: private at top level, and public inside a package.
private import TopLevelImports::*;
package Inner {
    public import TopLevelImports::*;
}
