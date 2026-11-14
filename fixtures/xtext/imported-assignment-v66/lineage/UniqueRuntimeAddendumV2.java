package intersect.oracle;
import java.io.*;import java.nio.charset.StandardCharsets;import java.util.*;
import org.eclipse.emf.common.util.URI;import org.eclipse.emf.ecore.*;
import org.eclipse.emf.common.util.Diagnostic;import org.eclipse.emf.ecore.util.Diagnostician;
import org.eclipse.xtext.*;import org.eclipse.xtext.resource.XtextResourceSet;
import org.eclipse.xtext.validation.*;import org.eclipse.xtext.util.CancelIndicator;

/** Verification-only addendum. Uses the same generated provider/setup and original model observer. */
public final class UniqueRuntimeAddendumV2 {
 static Map<String,Object> fields(Object...v){return TerminalNegationOracle.fields(v);}
 static Object failure(Throwable t){return FullRuntime.failure(t);}
 static Object diagnostic(Diagnostic d){List<Object>children=new ArrayList<>();for(var c:d.getChildren())children.add(diagnostic(c));return fields("severity",d.getSeverity(),"source",d.getSource(),"code",d.getCode(),"message",d.getMessage(),"children",children);}
 @SuppressWarnings("unchecked") public static void main(String[]args)throws Exception{
  EPackage provider=(EPackage)Class.forName(args[0]).getField("eINSTANCE").get(null);EPackage.Registry.INSTANCE.put(provider.getNsURI(),provider);
  var setup=(ISetup)Class.forName(args[1]+"StandaloneSetup").getConstructor().newInstance();var injector=setup.createInjectorAndDoEMFRegistration();String text=new String(Base64.getDecoder().decode(args[2]),StandardCharsets.UTF_8);
  Map<String,Object>result=fields("text",text,"feature",null,"direct_list_probe",null,"resource",null,"xtext_validation",null,"emf_validation",null);
  var cls=(EClass)provider.getEClassifier("Model");var ref=(EReference)cls.getEStructuralFeature("refs");var node=(EClass)provider.getEClassifier("Node");
  result.put("feature",fields("owner",MetamodelOracle.identity(cls),"name",ref.getName(),"type",MetamodelOracle.identity(ref.getEReferenceType()),"unique",ref.isUnique(),"ordered",ref.isOrdered(),"containment",ref.isContainment(),"lower",ref.getLowerBound(),"upper",ref.getUpperBound()));
  var isolated=provider.getEFactoryInstance().create(cls);var target=provider.getEFactoryInstance().create(node);target.eSet(node.getEStructuralFeature("name"),"a");var values=(List<Object>)isolated.eGet(ref);boolean first=values.add(target),second=values.add(target);result.put("direct_list_probe",fields("first_add_changed",first,"second_same_target_add_changed",second,"size",values.size(),"identical_targets",values.stream().allMatch(v->v==target),"scope","Isolated generated-list mutation probe; not a structurally valid complete resource/model."));
  org.eclipse.emf.ecore.resource.Resource resource=null;
  try{var resources=injector.getInstance(XtextResourceSet.class);resource=resources.createResource(URI.createURI("memory:/model.lexicaltest"));resource.load(new ByteArrayInputStream(text.getBytes(StandardCharsets.UTF_8)),null);var root=resource.getContents().isEmpty()?null:resource.getContents().get(0);Object model=RuntimeOracle.model(root);List<Object>errors=new ArrayList<>();for(var d:resource.getErrors())errors.add(fields("message",d.getMessage(),"line",d.getLine(),"column",d.getColumn()));result.put("resource",fields("model_after_resolving_observation",model,"errors",errors,"contents",resource.getContents().size(),"failure",null));
   try{List<Object>issues=new ArrayList<>();for(var i:injector.getInstance(IResourceValidator.class).validate(resource,CheckMode.ALL,CancelIndicator.NullImpl))issues.add(fields("severity",String.valueOf(i.getSeverity()),"code",i.getCode(),"message",i.getMessage(),"offset",i.getOffset(),"length",i.getLength(),"line",i.getLineNumber(),"column",i.getColumn()));result.put("xtext_validation",fields("issues",issues,"failure",null));}catch(Throwable t){result.put("xtext_validation",fields("failure",failure(t)));}
   try{result.put("emf_validation",root==null?null:diagnostic(Diagnostician.INSTANCE.validate(root)));}catch(Throwable t){result.put("emf_validation",fields("failure",failure(t)));}
  }catch(Throwable t){result.put("resource",fields("failure",failure(t)));}
  System.out.println(TerminalNegationOracle.json(result));
 }
}
