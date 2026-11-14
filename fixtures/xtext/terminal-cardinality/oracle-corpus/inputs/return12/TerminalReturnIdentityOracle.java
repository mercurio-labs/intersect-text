// Verification-only source-loaded return metadata. No native semantic substitute.
package intersect.oracle;

import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.EClass;
import org.eclipse.emf.ecore.EClassifier;
import org.eclipse.emf.ecore.EDataType;
import org.eclipse.emf.ecore.EEnum;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.emf.ecore.EPackage;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.xtext.AbstractRule;
import org.eclipse.xtext.GeneratedMetamodel;
import org.eclipse.xtext.Grammar;
import org.eclipse.xtext.GrammarUtil;
import org.eclipse.xtext.ParserRule;
import org.eclipse.xtext.ReferencedMetamodel;
import org.eclipse.xtext.RuleCall;
import org.eclipse.xtext.TerminalRule;
import org.eclipse.xtext.TypeRef;
import org.eclipse.xtext.XtextStandaloneSetup;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.eclipse.xtext.resource.XtextResource;
import org.eclipse.xtext.resource.XtextResourceSet;

public final class TerminalReturnIdentityOracle {
  static Map<String,Object> fields(Object... values) { return TerminalNegationOracle.fields(values); }
  static Object failure(Throwable value) { return fields("class", value.getClass().getName(), "message", value.getMessage()); }
  static String owner(AbstractRule rule) {
    if (rule == null || rule.eIsProxy()) return null;
    var grammar = GrammarUtil.getGrammar(rule);
    return grammar == null ? null : grammar.getName() + "::" + rule.getName();
  }
  static Object location(EObject value) {
    if (value == null) return null;
    var node = NodeModelUtils.getNode(value);
    return node == null ? null : fields("start", node.getOffset(), "end", node.getEndOffset(), "text", NodeModelUtils.getTokenText(node));
  }
  static Object resource(EObject value) {
    return value == null || value.eResource() == null ? null : value.eResource().getURI().toString();
  }
  static Object classifier(EClassifier value, Map<String,Grammar> grammars, Map<String,Object> identities) {
    if (value == null) return null;
    var pkg = value.getEPackage();
    List<Object> declarations = new ArrayList<>();
    if (pkg != null) for (var item : grammars.entrySet()) for (var declaration : item.getValue().getMetamodelDeclarations()) {
      if (declaration.getEPackage() == pkg) declarations.add(fields("grammar", item.getKey(), "source", identities.get(item.getKey()), "alias", declaration.getAlias(), "kind", declaration instanceof GeneratedMetamodel ? "generated" : declaration instanceof ReferencedMetamodel ? "imported" : declaration.eClass().getName()));
    }
    return fields("namespace", pkg == null ? null : pkg.getNsURI(), "name", value.getName(), "kind", value instanceof EEnum ? "EEnum" : value instanceof EClass ? "EClass" : value instanceof EDataType ? "EDataType" : value.eClass().getName(), "proxy", value.eIsProxy(), "uri", EcoreUtil.getURI(value).toString(), "resource_uri", resource(value), "instance_class_name", value.getInstanceClassName(), "metamodel_declarations", declarations);
  }
  public static void main(String[] args) throws Exception {
    Path root = Path.of(args[0]).toAbsolutePath();
    var injector = new XtextStandaloneSetup().createInjectorAndDoEMFRegistration();
    var set = injector.getInstance(XtextResourceSet.class);
    Map<String,Object> identities = new LinkedHashMap<>();
    Map<String,String> files = new LinkedHashMap<>();
    for (String line : Files.readAllLines(root.resolve("source-identities.tsv"), StandardCharsets.UTF_8)) {
      String[] parts = line.split("\t", -1);
      if (parts.length != 4) throw new IllegalArgumentException("Expected grammar,URI,SHA256,file");
      String actual = HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(root.resolve(parts[3]))));
      if (!actual.equals(parts[2])) throw new IllegalArgumentException("Authored source bytes differ: " + parts[0]);
      identities.put(parts[0], fields("uri", parts[1], "sha256", parts[2])); files.put(parts[0], parts[3]);
    }
    var imported = set.getResource(URI.createFileURI(root.resolve("imported.ecore").toString()), true);
    var pkg = (EPackage)imported.getContents().get(0);
    set.getPackageRegistry().put(pkg.getNsURI(), pkg);
    Map<String,Grammar> grammars = new LinkedHashMap<>();
    Map<String,XtextResource> resources = new LinkedHashMap<>();
    List<Object> syntax = new ArrayList<>();
    for (String name : Files.readAllLines(root.resolve("order.txt"), StandardCharsets.UTF_8)) {
      var loaded = (XtextResource)set.createResource(URI.createURI("memory:/" + name.replace('.', '/') + ".xtext"));
      loaded.load(new ByteArrayInputStream(Files.readAllBytes(root.resolve(files.get(name)))), null);
      resources.put(name, loaded);
      grammars.put(name, (Grammar)loaded.getContents().get(0));
      List<Object> errors = new ArrayList<>();
      for (var node : loaded.getParseResult().getSyntaxErrors()) errors.add(fields("start", node.getOffset(), "length", node.getLength(), "message", node.getSyntaxErrorMessage().getMessage()));
      syntax.add(fields("grammar", name, "source", identities.get(name), "errors", errors));
    }
    Object resolveFailure = null;
    try { EcoreUtil.resolveAll(set); } catch (Throwable error) { resolveFailure = failure(error); }
    List<Object> declared = new ArrayList<>(), calls = new ArrayList<>(), diagnostics = new ArrayList<>(), metamodels = new ArrayList<>();
    for (var item : grammars.entrySet()) {
      Grammar grammar = item.getValue();
      for (var declaration : grammar.getMetamodelDeclarations()) {
        var selected = declaration.getEPackage();
        metamodels.add(fields("grammar", item.getKey(), "source", identities.get(item.getKey()), "alias", declaration.getAlias(), "kind", declaration instanceof GeneratedMetamodel ? "generated" : declaration instanceof ReferencedMetamodel ? "imported" : declaration.eClass().getName(), "namespace", selected == null ? null : selected.getNsURI(), "proxy", selected == null ? null : Boolean.valueOf(selected.eIsProxy()), "resource_uri", resource(selected)));
      }
      for (AbstractRule rule : grammar.getRules()) {
        Object fragment = null;
        if (rule instanceof ParserRule) fragment = Boolean.valueOf(((ParserRule)rule).isFragment());
        else if (rule instanceof TerminalRule) fragment = Boolean.valueOf(((TerminalRule)rule).isFragment());
        TypeRef ref = rule.getType();
        Object selected = null, refFailure = null;
        try { selected = classifier(ref == null ? null : ref.getClassifier(), grammars, identities); } catch (Throwable error) { refFailure = failure(error); }
        declared.add(fields("identity", owner(rule), "source", identities.get(item.getKey()), "rule_kind", rule.eClass().getName(), "fragment", fragment, "rule_source_node", location(rule), "type_ref_present", ref != null, "type_source_node", location(ref), "type_metamodel_alias", ref == null || ref.getMetamodel() == null ? null : ref.getMetamodel().getAlias(), "type_metamodel_kind", ref == null || ref.getMetamodel() == null ? null : ref.getMetamodel().eClass().getName(), "classifier", selected, "type_failure", refFailure));
      }
      var contents = resources.get(item.getKey()).getAllContents();
      while (contents.hasNext()) {
        var value = contents.next();
        if (value instanceof RuleCall) {
          var call = (RuleCall)value;
          var target = call.getRule();
          var targetGrammar = target == null || target.eIsProxy() ? null : GrammarUtil.getGrammar(target);
          calls.add(fields("grammar", item.getKey(), "source", identities.get(item.getKey()), "source_node", location(call), "target", owner(target), "target_source", targetGrammar == null ? null : identities.get(targetGrammar.getName()), "proxy", target == null ? null : Boolean.valueOf(target.eIsProxy()), "explicit", call.isExplicitlyCalled()));
        }
      }
      List<Object> errors = new ArrayList<>();
      for (var error : resources.get(item.getKey()).getErrors()) errors.add(fields("line", error.getLine(), "column", error.getColumn(), "message", error.getMessage()));
      diagnostics.add(fields("grammar", item.getKey(), "errors", errors));
    }
    String entry = Files.readString(root.resolve("entry.txt"), StandardCharsets.UTF_8).trim();
    List<Object> effective = new ArrayList<>(); Object effectiveFailure = null;
    try { for (var rule : GrammarUtil.allRules(grammars.get(entry))) effective.add(owner(rule)); } catch (Throwable error) { effectiveFailure = failure(error); }
    System.out.println(TerminalNegationOracle.json(fields("entry", entry, "syntax", syntax, "resolve_failure", resolveFailure, "linking", diagnostics, "metamodels", metamodels, "declared_rules", declared, "calls", calls, "effective_rules", effective, "effective_failure", effectiveFailure)));
  }
}
