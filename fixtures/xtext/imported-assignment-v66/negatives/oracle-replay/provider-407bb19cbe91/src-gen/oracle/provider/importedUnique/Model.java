/**
 */
package oracle.provider.importedUnique;

import org.eclipse.emf.common.util.EList;

import org.eclipse.emf.ecore.EObject;

/**
 * <!-- begin-user-doc -->
 * A representation of the model object '<em><b>Model</b></em>'.
 * <!-- end-user-doc -->
 *
 * <p>
 * The following features are supported:
 * </p>
 * <ul>
 *   <li>{@link oracle.provider.importedUnique.Model#getNodes <em>Nodes</em>}</li>
 *   <li>{@link oracle.provider.importedUnique.Model#getRefs <em>Refs</em>}</li>
 * </ul>
 *
 * @see oracle.provider.importedUnique.ImportedUniquePackage#getModel()
 * @model
 * @generated
 */
public interface Model extends EObject {
	/**
	 * Returns the value of the '<em><b>Nodes</b></em>' containment reference list.
	 * The list contents are of type {@link oracle.provider.importedUnique.Node}.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the value of the '<em>Nodes</em>' containment reference list.
	 * @see oracle.provider.importedUnique.ImportedUniquePackage#getModel_Nodes()
	 * @model containment="true" required="true"
	 * @generated
	 */
	EList<Node> getNodes();

	/**
	 * Returns the value of the '<em><b>Refs</b></em>' reference list.
	 * The list contents are of type {@link oracle.provider.importedUnique.Node}.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the value of the '<em>Refs</em>' reference list.
	 * @see oracle.provider.importedUnique.ImportedUniquePackage#getModel_Refs()
	 * @model required="true" changeable="false"
	 * @generated
	 */
	EList<Node> getRefs();

} // Model
