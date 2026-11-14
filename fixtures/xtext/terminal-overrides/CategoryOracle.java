// Verification-only adapter to the pinned datatype override checker.
package intersect.oracle;
import java.lang.reflect.*;
import org.eclipse.emf.ecore.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.xtext.ecoreInference.Xtext2EcoreTransformer;
public final class CategoryOracle {
 static AbstractRule rule(Grammar g,String kind,EClassifier type) {
  AbstractRule r=kind.equals("parser")?XtextFactory.eINSTANCE.createParserRule():kind.equals("enum")?XtextFactory.eINSTANCE.createEnumRule():XtextFactory.eINSTANCE.createTerminalRule();
  r.setName("Value");
  if(type!=null) {TypeRef t=XtextFactory.eINSTANCE.createTypeRef();t.setClassifier(type);r.setType(t);}
  if(kind.equals("fragment"))((TerminalRule)r).setFragment(true);
  g.getRules().add(r);return r;
 }
 static void check(String parent,String child,String scenario,Method method) throws Exception {
  Grammar base=BindingOracle.grammar("audit.Base"),g=BindingOracle.grammar("audit.Child");g.getUsedGrammars().add(base);
  EEnum common=EcoreFactory.eINSTANCE.createEEnum();common.setName("Shared");
  rule(base,parent,scenario.equals("missing-type")?null:common);
  AbstractRule replacement=rule(g,child,scenario.equals("different-type")?EcorePackage.Literals.ESTRING:common);
  boolean accepted=true;
  try {method.invoke(new Xtext2EcoreTransformer(g),replacement);}
  catch(InvocationTargetException e) {
   if(!e.getCause().getClass().getSimpleName().equals("TransformationException"))throw e;
   accepted=false;
  }
  System.out.println("{\"parent\":\""+parent+"\",\"child\":\""+child+"\",\"case\":\""+scenario+"\",\"accepted\":"+accepted+"}");
 }
 public static void main(String[] args) throws Exception {
  Method method=Xtext2EcoreTransformer.class.getDeclaredMethod("checkSupertypeOfOverriddenDatatypeRule",AbstractRule.class);method.setAccessible(true);
  for(String parent:new String[]{"parser","enum","terminal"})for(String child:new String[]{"parser","enum","terminal"})check(parent,child,"same-type",method);
  check("parser","fragment","same-type",method);
  check("enum","fragment","same-type",method);
  check("parser","terminal","different-type",method);
  check("parser","terminal","missing-type",method);
 }
}
