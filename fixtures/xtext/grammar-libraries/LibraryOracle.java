// Intersect-authored verification adapter; no production Java dependency.
package intersect.oracle;
import java.nio.file.*;
import java.io.StringReader;
import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.IParser;
public final class LibraryOracle {
 public static void main(String[] args) throws Exception {
  IParser parser = new XtextStandaloneSetup().createInjectorAndDoEMFRegistration().getInstance(IParser.class);
  for (String kind : new String[]{"enum", "datatype", "fragment"}) {
   var parentResult = parser.parse(new StringReader(Files.readString(Path.of(args[0], kind + ".xtext"))));
   String childText = Files.readString(Path.of(args[0], "model.xtext"));
   if (kind.equals("fragment")) childText = childText.replace("shade=Choice ';'", "Tail");
   var childResult = parser.parse(new StringReader(childText));
   if (parentResult.hasSyntaxErrors() || childResult.hasSyntaxErrors()) throw new AssertionError(kind);
   Grammar parent = (Grammar)parentResult.getRootASTElement(), child = (Grammar)childResult.getRootASTElement();
   // IParser does not link proxies: supply the exact authored dependency explicitly.
   child.getUsedGrammars().clear(); child.getUsedGrammars().add(parent);
   String name = kind.equals("fragment") ? "Tail" : "Choice";
   AbstractRule selected = GrammarUtil.findRuleForName(child, name);
   if (selected == null || selected.eContainer() != parent) throw new AssertionError(kind);
   boolean fragment = selected instanceof ParserRule && ((ParserRule)selected).isFragment();
   System.out.println("{\"case\":\""+kind+"\",\"syntax_errors\":false,\"rule\":\""+name+"\",\"kind\":\""+selected.eClass().getName()+"\",\"fragment\":"+fragment+",\"declaring_grammar\":\""+parent.getName()+"\"}");
  }
 }
}
