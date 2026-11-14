// Intersect-authored verification of pinned metamodel selection and URI validation.
package intersect.oracle;
import org.eclipse.emf.ecore.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.xtext.XtextValidator;
import org.eclipse.xtext.xtext.ecoreInference.Xtext2EcoreTransformer;
public final class MetamodelOracle extends XtextValidator {
 private int errors;
 @Override protected void error(String m,EStructuralFeature f) {errors++;}
 static ReferencedMetamodel add(Grammar g,String uri,String alias,String classifier) {
  EPackage p=EcoreFactory.eINSTANCE.createEPackage();p.setName("test");p.setNsURI(uri);
  EClass c=EcoreFactory.eINSTANCE.createEClass();c.setName(classifier);p.getEClassifiers().add(c);
  ReferencedMetamodel d=XtextFactory.eINSTANCE.createReferencedMetamodel();d.setAlias(alias);d.setEPackage(p);g.getMetamodelDeclarations().add(d);return d;
 }
 public static void main(String[] args) {
  for(String scenario:new String[]{"shared-alias-unique","shared-alias-conflict","distinct-alias","missing-alias","missing-classifier","unaliased","duplicate-uri"}) {
   Grammar g=BindingOracle.grammar("audit.Imports");
   String alias=scenario.equals("unaliased")?null:"p";
   add(g,"urn:a",alias,"A");
   add(g,scenario.equals("duplicate-uri")?"urn:a":"urn:b",scenario.equals("distinct-alias")?"q":alias,scenario.equals("shared-alias-conflict")?"A":"B");
   String queryAlias=scenario.equals("missing-alias")?"missing":scenario.equals("unaliased")?"":"p";
   String queryType=scenario.equals("missing-classifier")?"Missing":"A";
   AbstractMetamodelDeclaration selected=new Xtext2EcoreTransformer(g).findMetamodel(g,queryAlias,queryType);
   MetamodelOracle v=new MetamodelOracle();for(AbstractMetamodelDeclaration d:g.getMetamodelDeclarations())v.checkMetamodelUris(d);
   System.out.println("{\"case\":\""+scenario+"\",\"selected\":"+(selected==null?"null":"\""+selected.getEPackage().getNsURI()+"\"")+",\"uri_errors\":"+v.errors+"}");
  }
 }
}
