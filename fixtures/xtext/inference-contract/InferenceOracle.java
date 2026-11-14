// Independently authored verification harness; production does not invoke Java.
package intersect.oracle;
import org.eclipse.xtext.xtext.ecoreInference.Xtext2EcoreTransformer;
import org.eclipse.xtext.*;
import org.eclipse.emf.ecore.*;

public final class InferenceOracle {
  static final XtextFactory F = XtextFactory.eINSTANCE;
  static Keyword keyword(String text) { Keyword k=F.createKeyword(); k.setValue(text); return k; }
  static RuleCall call(AbstractRule target) { RuleCall c=F.createRuleCall(); c.setRule(target); return c; }
  static ParserRule rule(Grammar g,String name,AbstractElement body) { ParserRule r=F.createParserRule(); r.setName(name); r.setAlternatives(body); g.getRules().add(r); return r; }
  static TypeRef type(EClassifier c) { TypeRef t=F.createTypeRef(); t.setClassifier(c); return t; }
  static void observe(String id,int variant) throws Exception {
    Grammar g=F.createGrammar(); g.setName("oracle.Inference");
    ReferencedMetamodel ec=F.createReferencedMetamodel(); ec.setAlias("ecore"); ec.setEPackage(EcorePackage.eINSTANCE); g.getMetamodelDeclarations().add(ec);
    EPackage model=EcoreFactory.eINSTANCE.createEPackage(); model.setName("collision"); model.setNsURI("urn:oracle:collision");
    EClass cls=EcoreFactory.eINSTANCE.createEClass(); cls.setName("Operator"); model.getEClassifiers().add(cls);
    ReferencedMetamodel mm=F.createReferencedMetamodel(); mm.setEPackage(model); g.getMetamodelDeclarations().add(mm);
    ParserRule r=rule(g,"Operator",keyword("if"));
    if(variant==1) r.setType(type(cls));
    if(variant==2) r.setType(type(EcorePackage.Literals.ESTRING));
    if(variant==3) { Assignment a=F.createAssignment(); a.setFeature("name"); a.setOperator("="); a.setTerminal(keyword("if")); r.setAlternatives(a); }
    if(variant==4) { Action a=F.createAction(); a.setType(type(cls)); r.setAlternatives(a); }
    if(variant==5) { ParserRule next=rule(g,"Later",keyword("if")); r.setAlternatives(call(next)); }
    if(variant==6) r.setAlternatives(call(null));
    if(variant==7) { Alternatives a=F.createAlternatives(); a.getElements().add(keyword("if")); a.getElements().add(call(r)); r.setAlternatives(a); }
    if(variant==8) r.getAlternatives().setCardinality("?");
    Xtext2EcoreTransformer transformer=new Xtext2EcoreTransformer(g);
    java.lang.reflect.Method method=Xtext2EcoreTransformer.class.getDeclaredMethod("getOrComputeReturnType",AbstractRule.class); method.setAccessible(true);
    TypeRef result=(TypeRef)method.invoke(transformer,r);
    EClassifier c=result.getClassifier();
    System.out.println("{\"case\":\""+id+"\",\"classifier\":\""+(c==null?"unresolved":c.getEPackage().getNsURI()+"#//"+c.getName())+"\"}");
  }
  public static void main(String[] args) throws Exception {
    String[] cases={"omitted_literal_with_same_named_class","explicit_model_class","explicit_estring","assignment_is_model","action_is_model","forward_scalar_call","unresolved_call_is_not_scalar","recursive_scalar_classification","optional_scalar_classification"};
    for(int i=0;i<cases.length;i++) observe(cases[i],i);
  }
}
