// Intersect-authored verification adapter. Production never uses Java.
package intersect.oracle;
import org.eclipse.xtext.*;
import org.eclipse.xtext.xtext.XtextLinker;
public final class BindingOracle extends XtextLinker {
 static Grammar grammar(String name) { Grammar g=XtextFactory.eINSTANCE.createGrammar();g.setName(name);return g; }
 static TerminalRule terminal(Grammar g,String name,boolean fragment,String text) {
  TerminalRule t=XtextFactory.eINSTANCE.createTerminalRule();t.setName(name);t.setFragment(fragment);
  Keyword k=XtextFactory.eINSTANCE.createKeyword();k.setValue(text);t.setAlternatives(k);g.getRules().add(t);return t;
 }
 static RuleCall call(AbstractRule target,boolean explicit) {RuleCall c=XtextFactory.eINSTANCE.createRuleCall();c.setRule(target);c.setExplicitlyCalled(explicit);return c;}
 static String identity(AbstractRule r) {return GrammarUtil.getGrammar(r).getName()+"::"+r.getName();}
 public static void main(String[] args) {
  for(String scenario:new String[]{"terminal","fragment","promoted-fragment","explicit-parent-call","three-level"}) {
   Grammar base=grammar("audit.Base"),child=grammar("audit.Child");child.getUsedGrammars().add(base);
   boolean fragment=!scenario.equals("terminal");
   TerminalRule old=terminal(base,"LETTER",fragment,"a");
   TerminalRule replacement=terminal(child,"LETTER",scenario.equals("fragment")||scenario.equals("three-level"),"b");
   TerminalRule value=terminal(base,"VALUE",false,"@");
   RuleCall nested=call(old,scenario.equals("explicit-parent-call"));
   Group group=XtextFactory.eINSTANCE.createGroup();group.getElements().add(value.getAlternatives());group.getElements().add(nested);value.setAlternatives(group);
   ParserRule model=XtextFactory.eINSTANCE.createParserRule();model.setName("Model");
   RuleCall parserCall=call(old,false);model.setAlternatives(parserCall);base.getRules().add(model);
   base.setDefinesHiddenTokens(true);base.getHiddenTokens().add(old);
   BindingOracle oracle=new BindingOracle();oracle.updateOverriddenRules(child);
   if(scenario.equals("three-level")) {Grammar top=grammar("audit.Top");top.getUsedGrammars().add(child);replacement=terminal(top,"LETTER",true,"c");oracle.updateOverriddenRules(top);}
   System.out.println("{\"case\":\""+scenario+"\",\"terminal_call\":\""+identity(nested.getRule())+"\",\"parser_call\":\""+identity(parserCall.getRule())+"\",\"hidden\":\""+identity(base.getHiddenTokens().get(0))+"\",\"selected_literal\":\""+((Keyword)nested.getRule().getAlternatives()).getValue()+"\"}");
  }
 }
}
