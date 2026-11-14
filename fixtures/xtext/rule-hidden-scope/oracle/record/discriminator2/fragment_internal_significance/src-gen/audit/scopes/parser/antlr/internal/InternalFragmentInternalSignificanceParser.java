package audit.scopes.parser.antlr.internal;

import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.*;
import org.eclipse.xtext.parser.impl.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.xtext.parser.antlr.AbstractInternalAntlrParser;
import org.eclipse.xtext.parser.antlr.XtextTokenStream;
import org.eclipse.xtext.parser.antlr.XtextTokenStream.HiddenTokens;
import org.eclipse.xtext.parser.antlr.AntlrDatatypeRuleToken;
import audit.scopes.services.FragmentInternalSignificanceGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalFragmentInternalSignificanceParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_ID", "RULE_WS", "RULE_ML_COMMENT", "RULE_SL_COMMENT", "'['", "']'", "'done'", "':'"
    };
    public static final int T__9=9;
    public static final int T__8=8;
    public static final int RULE_ID=4;
    public static final int RULE_WS=5;
    public static final int RULE_SL_COMMENT=7;
    public static final int T__11=11;
    public static final int RULE_ML_COMMENT=6;
    public static final int EOF=-1;
    public static final int T__10=10;

    // delegates
    // delegators


        public InternalFragmentInternalSignificanceParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalFragmentInternalSignificanceParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalFragmentInternalSignificanceParser.tokenNames; }
    public String getGrammarFileName() { return "InternalFragmentInternalSignificance.g"; }



     	private FragmentInternalSignificanceGrammarAccess grammarAccess;

        public InternalFragmentInternalSignificanceParser(TokenStream input, FragmentInternalSignificanceGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected FragmentInternalSignificanceGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalFragmentInternalSignificance.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalFragmentInternalSignificance.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalFragmentInternalSignificance.g:65:2: iv_ruleModel= ruleModel EOF
            {
             newCompositeNode(grammarAccess.getModelRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleModel=ruleModel();

            state._fsp--;

             current =iv_ruleModel; 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "entryRuleModel"


    // $ANTLR start "ruleModel"
    // InternalFragmentInternalSignificance.g:71:1: ruleModel returns [EObject current=null] : ( () otherlv_1= '[' this_Fields_2= ruleFields[$current] otherlv_3= ']' otherlv_4= 'done' ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        Token otherlv_1=null;
        Token otherlv_3=null;
        Token otherlv_4=null;
        EObject this_Fields_2 = null;



        	enterRule();

        try {
            // InternalFragmentInternalSignificance.g:77:2: ( ( () otherlv_1= '[' this_Fields_2= ruleFields[$current] otherlv_3= ']' otherlv_4= 'done' ) )
            // InternalFragmentInternalSignificance.g:78:2: ( () otherlv_1= '[' this_Fields_2= ruleFields[$current] otherlv_3= ']' otherlv_4= 'done' )
            {
            // InternalFragmentInternalSignificance.g:78:2: ( () otherlv_1= '[' this_Fields_2= ruleFields[$current] otherlv_3= ']' otherlv_4= 'done' )
            // InternalFragmentInternalSignificance.g:79:3: () otherlv_1= '[' this_Fields_2= ruleFields[$current] otherlv_3= ']' otherlv_4= 'done'
            {
            // InternalFragmentInternalSignificance.g:79:3: ()
            // InternalFragmentInternalSignificance.g:80:4: 
            {

            				current = forceCreateModelElement(
            					grammarAccess.getModelAccess().getModelAction_0(),
            					current);
            			

            }

            otherlv_1=(Token)match(input,8,FOLLOW_3); 

            			newLeafNode(otherlv_1, grammarAccess.getModelAccess().getLeftSquareBracketKeyword_1());
            		

            			if (current==null) {
            				current = createModelElement(grammarAccess.getModelRule());
            			}
            			newCompositeNode(grammarAccess.getModelAccess().getFieldsParserRuleCall_2());
            		
            pushFollow(FOLLOW_4);
            this_Fields_2=ruleFields(current);

            state._fsp--;


            			current = this_Fields_2;
            			afterParserOrEnumRuleCall();
            		
            otherlv_3=(Token)match(input,9,FOLLOW_5); 

            			newLeafNode(otherlv_3, grammarAccess.getModelAccess().getRightSquareBracketKeyword_3());
            		
            otherlv_4=(Token)match(input,10,FOLLOW_2); 

            			newLeafNode(otherlv_4, grammarAccess.getModelAccess().getDoneKeyword_4());
            		

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
    // $ANTLR end "ruleModel"


    // $ANTLR start "ruleFields"
    // InternalFragmentInternalSignificance.g:114:1: ruleFields[EObject in_current] returns [EObject current=in_current] : ( ( (lv_name_0_0= RULE_ID ) ) otherlv_1= ':' this_ID_2= RULE_ID ) ;
    public final EObject ruleFields(EObject in_current) throws RecognitionException {
        EObject current = in_current;

        Token lv_name_0_0=null;
        Token otherlv_1=null;
        Token this_ID_2=null;


        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalFragmentInternalSignificance.g:121:2: ( ( ( (lv_name_0_0= RULE_ID ) ) otherlv_1= ':' this_ID_2= RULE_ID ) )
            // InternalFragmentInternalSignificance.g:122:2: ( ( (lv_name_0_0= RULE_ID ) ) otherlv_1= ':' this_ID_2= RULE_ID )
            {
            // InternalFragmentInternalSignificance.g:122:2: ( ( (lv_name_0_0= RULE_ID ) ) otherlv_1= ':' this_ID_2= RULE_ID )
            // InternalFragmentInternalSignificance.g:123:3: ( (lv_name_0_0= RULE_ID ) ) otherlv_1= ':' this_ID_2= RULE_ID
            {
            // InternalFragmentInternalSignificance.g:123:3: ( (lv_name_0_0= RULE_ID ) )
            // InternalFragmentInternalSignificance.g:124:4: (lv_name_0_0= RULE_ID )
            {
            // InternalFragmentInternalSignificance.g:124:4: (lv_name_0_0= RULE_ID )
            // InternalFragmentInternalSignificance.g:125:5: lv_name_0_0= RULE_ID
            {
            lv_name_0_0=(Token)match(input,RULE_ID,FOLLOW_6); 

            					newLeafNode(lv_name_0_0, grammarAccess.getFieldsAccess().getNameIDTerminalRuleCall_0_0());
            				

            					if (current==null) {
            						current = createModelElement(grammarAccess.getFieldsRule());
            					}
            					setWithLastConsumed(
            						current,
            						"name",
            						lv_name_0_0,
            						"audit.scopes.FragmentInternalSignificance.ID");
            				

            }


            }

            otherlv_1=(Token)match(input,11,FOLLOW_3); 

            			newLeafNode(otherlv_1, grammarAccess.getFieldsAccess().getColonKeyword_1());
            		
            this_ID_2=(Token)match(input,RULE_ID,FOLLOW_2); 

            			newLeafNode(this_ID_2, grammarAccess.getFieldsAccess().getIDTerminalRuleCall_2());
            		

            }


            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {

            	myHiddenTokenState.restore();

        }
        return current;
    }
    // $ANTLR end "ruleFields"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000000L});
    public static final BitSet FOLLOW_2 = new BitSet(new long[]{0x0000000000000002L});
    public static final BitSet FOLLOW_3 = new BitSet(new long[]{0x0000000000000010L});
    public static final BitSet FOLLOW_4 = new BitSet(new long[]{0x0000000000000200L});
    public static final BitSet FOLLOW_5 = new BitSet(new long[]{0x0000000000000400L});
    public static final BitSet FOLLOW_6 = new BitSet(new long[]{0x0000000000000800L});

}