/**
 */
package oracle.provider.colors;

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
 *   <li>{@link oracle.provider.colors.Model#getShade <em>Shade</em>}</li>
 * </ul>
 *
 * @see oracle.provider.colors.ColorsPackage#getModel()
 * @model
 * @generated
 */
public interface Model extends EObject {
	/**
	 * Returns the value of the '<em><b>Shade</b></em>' attribute.
	 * The literals are from the enumeration {@link oracle.provider.colors.Shade}.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the value of the '<em>Shade</em>' attribute.
	 * @see oracle.provider.colors.Shade
	 * @see #setShade(Shade)
	 * @see oracle.provider.colors.ColorsPackage#getModel_Shade()
	 * @model
	 * @generated
	 */
	Shade getShade();

	/**
	 * Sets the value of the '{@link oracle.provider.colors.Model#getShade <em>Shade</em>}' attribute.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @param value the new value of the '<em>Shade</em>' attribute.
	 * @see oracle.provider.colors.Shade
	 * @see #getShade()
	 * @generated
	 */
	void setShade(Shade value);

} // Model
