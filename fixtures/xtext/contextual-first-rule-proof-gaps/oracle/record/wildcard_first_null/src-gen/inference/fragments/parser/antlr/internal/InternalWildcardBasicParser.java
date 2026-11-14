package inference.fragments.parser.antlr.internal;

import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.*;
import org.eclipse.xtext.parser.impl.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.xtext.parser.antlr.AbstractInternalAntlrParser;
import org.eclipse.xtext.parser.antlr.XtextTokenStream;
import org.eclipse.xtext.parser.antlr.XtextTokenStream.HiddenTokens;
import org.eclipse.xtext.parser.antlr.AntlrDatatypeRuleToken;
import inference.fragments.services.WildcardBasicGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalWildcardBasicParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_ID", "RULE_WS"
    };
    public static final int RULE_ID=4;
    public static final int RULE_WS=5;
    public static final int EOF=-1;

    // delegates
    // delegators


        public InternalWildcardBasicParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalWildcardBasicParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalWildcardBasicParser.tokenNames; }
    public String getGrammarFileName() { return "InternalWildcardBasic.g"; }



     	private WildcardBasicGrammarAccess grammarAccess;

        public InternalWildcardBasicParser(TokenStream input, WildcardBasicGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Fields";
       	}

       	@Override
       	protected WildcardBasicGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "ruleFields"
    // InternalWildcardBasic.g:65:1: ruleFields[EObject in_current] returns [EObject current=in_current] : ( (lv_name_0_0= RULE_ID ) ) ;
    public final EObject ruleFields(EObject in_current) throws RecognitionException {
        EObject current = in_current;

        Token lv_name_0_0=null;


        	enterRule();

        try {
            // InternalWildcardBasic.g:71:2: ( ( (lv_name_0_0= RULE_ID ) ) )
            // InternalWildcardBasic.g:72:2: ( (lv_name_0_0= RULE_ID ) )
            {
            // InternalWildcardBasic.g:72:2: ( (lv_name_0_0= RULE_ID ) )
            // InternalWildcardBasic.g:73:3: (lv_name_0_0= RULE_ID )
            {
            // InternalWildcardBasic.g:73:3: (lv_name_0_0= RULE_ID )
            // InternalWildcardBasic.g:74:4: lv_name_0_0= RULE_ID
            {
            lv_name_0_0=(Token)match(input,RULE_ID,FOLLOW_1); 

            				newLeafNode(lv_name_0_0, grammarAccess.getFieldsAccess().getNameIDTerminalRuleCall_0());
            			

            				if (current==null) {
            					current = createModelElement(grammarAccess.getFieldsRule());
            				}
            				setWithLastConsumed(
            					current,
            					"name",
            					lv_name_0_0,
            					"inference.fragments.WildcardBasic.ID");
            			

            }


            }


            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "ruleFields"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000002L});

}