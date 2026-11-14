/**
 */
package oracle.provider.firstRuleIdentity;

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
 *   <li>{@link oracle.provider.firstRuleIdentity.Model#getName <em>Name</em>}</li>
 *   <li>{@link oracle.provider.firstRuleIdentity.Model#getMembers <em>Members</em>}</li>
 * </ul>
 *
 * @see oracle.provider.firstRuleIdentity.FirstRuleIdentityPackage#getModel()
 * @model
 * @generated
 */
public interface Model extends EObject {
	/**
	 * Returns the value of the '<em><b>Name</b></em>' attribute.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the value of the '<em>Name</em>' attribute.
	 * @see #setName(String)
	 * @see oracle.provider.firstRuleIdentity.FirstRuleIdentityPackage#getModel_Name()
	 * @model
	 * @generated
	 */
	String getName();

	/**
	 * Sets the value of the '{@link oracle.provider.firstRuleIdentity.Model#getName <em>Name</em>}' attribute.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @param value the new value of the '<em>Name</em>' attribute.
	 * @see #getName()
	 * @generated
	 */
	void setName(String value);

	/**
	 * Returns the value of the '<em><b>Members</b></em>' containment reference list.
	 * The list contents are of type {@link oracle.provider.firstRuleIdentity.Element}.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the value of the '<em>Members</em>' containment reference list.
	 * @see oracle.provider.firstRuleIdentity.FirstRuleIdentityPackage#getModel_Members()
	 * @model containment="true"
	 * @generated
	 */
	EList<Element> getMembers();

} // Model
