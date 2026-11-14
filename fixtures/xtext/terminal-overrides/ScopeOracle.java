// Intersect-authored adapter to pinned Xtext's explicit-call scope; verification only.
package intersect.oracle;
import org.eclipse.xtext.*;
import org.eclipse.xtext.naming.QualifiedName;
import org.eclipse.xtext.xtext.SuperCallScope;
public final class ScopeOracle {
 public static void main(String[] args) {
  var parser=new XtextStandaloneSetup().createInjectorAndDoEMFRegistration().getInstance(org.eclipse.xtext.parser.IParser.class);
  for(String spelling:new String[]{"super","super::LETTER","Base::LETTER","audit::Base::LETTER","Child::LETTER","audit::Child::LETTER","Missing::LETTER","Base::MISSING","audit.Base.LETTER"}) {
   Grammar base=BindingOracle.grammar("audit.Base"),child=BindingOracle.grammar("audit.Child");child.getUsedGrammars().add(base);
   TerminalRule old=BindingOracle.terminal(base,"LETTER",true,"a"),replacement=BindingOracle.terminal(child,"LETTER",true,"b");
   RuleCall context=BindingOracle.call(old,false);replacement.setAlternatives(context);
   var description=new SuperCallScope(context).getSingleElement(QualifiedName.create(spelling.split("::")));
   AbstractRule selected=description==null?null:(AbstractRule)description.getEObjectOrProxy();
   var parsed=parser.parse(new java.io.StringReader("grammar audit.Child terminal LETTER: "+spelling+";"));
   if(parsed.hasSyntaxErrors())throw new IllegalStateException("Unexpected syntax error: "+spelling);
   AbstractElement body=((Grammar)parsed.getRootASTElement()).getRules().get(0).getAlternatives();
   System.out.println("{\"spelling\":\""+spelling+"\",\"selected\":"+(selected==null?"null":"\""+BindingOracle.identity(selected)+"\"")+",\"explicit\":"+context.isExplicitlyCalled()+",\"body_kind\":\""+body.eClass().getName()+"\"}");
  }
 }
}
