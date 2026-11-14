// Intersect-authored adapter; invokes pinned validation, not production code.
package intersect.oracle;
import org.eclipse.emf.ecore.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.xtext.XtextValidator;
public final class FinalOracle extends XtextValidator {
 private int errors, warnings;
 @Override protected void error(String m,EObject o,EStructuralFeature f,String c,String... data) {errors++;}
 @Override protected void warning(String m,EObject o,EStructuralFeature f,String c,String... data) {warnings++;}
 static AbstractRule rule(Grammar g,String kind) {
  AbstractRule r=kind.equals("parser")?XtextFactory.eINSTANCE.createParserRule():kind.equals("terminal")?XtextFactory.eINSTANCE.createTerminalRule():XtextFactory.eINSTANCE.createEnumRule();
  r.setName("Value");g.getRules().add(r);return r;
 }
 static void annotate(AbstractRule r,String name) {Annotation a=XtextFactory.eINSTANCE.createAnnotation();a.setName(name);r.getAnnotations().add(a);}
 public static void main(String[] args) {
  for(String kind:new String[]{"parser","terminal","enum"}) {
   for(String scenario:new String[]{"ordinary","final","final-unannotated","final-ancestor"}) {
    Grammar base=BindingOracle.grammar("audit.Base"),child=BindingOracle.grammar("audit.Child");
    AbstractRule original=rule(base,kind),replacement=rule(child,kind);
    if(!scenario.equals("ordinary"))annotate(original,"Final");
    if(!scenario.equals("final-unannotated"))annotate(replacement,"Override");
    if(scenario.equals("final-ancestor")) {Grammar middle=BindingOracle.grammar("audit.Middle");middle.getUsedGrammars().add(base);child.getUsedGrammars().add(middle);}
    else child.getUsedGrammars().add(base);
    FinalOracle oracle=new FinalOracle();oracle.checkOverridingRule(replacement);
    System.out.println("{\"kind\":\""+kind+"\",\"case\":\""+scenario+"\",\"errors\":"+oracle.errors+",\"warnings\":"+oracle.warnings+"}");
   }
  }
 }
}
