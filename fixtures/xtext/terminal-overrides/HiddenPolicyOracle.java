// Intersect-authored verification adapter; no production Java dependency.
package intersect.oracle;
import org.eclipse.xtext.*;
import org.eclipse.xtext.xtext.*;
public final class HiddenPolicyOracle {
 public static void main(String[] args) {
  for(String scenario:new String[]{"absent-root","empty-root","hidden-root","inherited-absent","inherited-hidden","child-empty"}) {
   Grammar base=BindingOracle.grammar("audit.Base"), child=BindingOracle.grammar("audit.Child");
   ParserRule model=XtextFactory.eINSTANCE.createParserRule();model.setName("Model");
   Keyword keyword=XtextFactory.eINSTANCE.createKeyword();keyword.setValue("model");model.setAlternatives(keyword);base.getRules().add(model);
   TerminalRule ws=BindingOracle.terminal(base,"WS",false," ");
   if(scenario.equals("empty-root")) base.setDefinesHiddenTokens(true);
   if(scenario.equals("hidden-root")||scenario.equals("inherited-hidden")||scenario.equals("child-empty")) {
    base.setDefinesHiddenTokens(true);base.getHiddenTokens().add(ws);
   }
   boolean inherited=scenario.startsWith("inherited-")||scenario.equals("child-empty");
   Grammar selected=base;
   if(inherited) {child.getUsedGrammars().add(base);selected=child;}
   if(scenario.equals("child-empty")) child.setDefinesHiddenTokens(true);
   Grammar flat=new FlattenedGrammarAccess(new RuleNames(selected,true),new RuleFilter()).getFlattenedGrammar();
   String tokens=flat.getHiddenTokens().stream().map(r->"\""+r.getName()+"\"").collect(java.util.stream.Collectors.joining(","));
   System.out.println("{\"case\":\""+scenario+"\",\"source_defines_hidden\":"+selected.isDefinesHiddenTokens()+",\"flattened_defines_hidden\":"+flat.isDefinesHiddenTokens()+",\"hidden\":["+tokens+"]}");
  }
 }
}
