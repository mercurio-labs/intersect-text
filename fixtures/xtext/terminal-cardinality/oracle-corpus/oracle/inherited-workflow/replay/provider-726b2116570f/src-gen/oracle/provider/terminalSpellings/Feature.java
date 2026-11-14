/**
 */
package oracle.provider.terminalSpellings;


/**
 * <!-- begin-user-doc -->
 * A representation of the model object '<em><b>Feature</b></em>'.
 * <!-- end-user-doc -->
 *
 * <p>
 * The following features are supported:
 * </p>
 * <ul>
 *   <li>{@link oracle.provider.terminalSpellings.Feature#getTarget <em>Target</em>}</li>
 * </ul>
 *
 * @see oracle.provider.terminalSpellings.TerminalSpellingsPackage#getFeature()
 * @model
 * @generated
 */
public interface Feature extends Element {
	/**
	 * Returns the value of the '<em><b>Target</b></em>' reference.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the value of the '<em>Target</em>' reference.
	 * @see #isSetTarget()
	 * @see #unsetTarget()
	 * @see #setTarget(Type)
	 * @see oracle.provider.terminalSpellings.TerminalSpellingsPackage#getFeature_Target()
	 * @model unsettable="true"
	 * @generated
	 */
	Type getTarget();

	/**
	 * Sets the value of the '{@link oracle.provider.terminalSpellings.Feature#getTarget <em>Target</em>}' reference.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @param value the new value of the '<em>Target</em>' reference.
	 * @see #isSetTarget()
	 * @see #unsetTarget()
	 * @see #getTarget()
	 * @generated
	 */
	void setTarget(Type value);

	/**
	 * Unsets the value of the '{@link oracle.provider.terminalSpellings.Feature#getTarget <em>Target</em>}' reference.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see #isSetTarget()
	 * @see #getTarget()
	 * @see #setTarget(Type)
	 * @generated
	 */
	void unsetTarget();

	/**
	 * Returns whether the value of the '{@link oracle.provider.terminalSpellings.Feature#getTarget <em>Target</em>}' reference is set.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return whether the value of the '<em>Target</em>' reference is set.
	 * @see #unsetTarget()
	 * @see #getTarget()
	 * @see #setTarget(Type)
	 * @generated
	 */
	boolean isSetTarget();

} // Feature
