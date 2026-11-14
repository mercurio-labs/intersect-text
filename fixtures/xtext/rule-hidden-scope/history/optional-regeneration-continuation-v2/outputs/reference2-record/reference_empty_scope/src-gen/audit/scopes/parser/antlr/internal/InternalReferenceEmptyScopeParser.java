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
import audit.scopes.services.ReferenceEmptyScopeGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalReferenceEmptyScopeParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_ID", "RULE_WS", "RULE_ML_COMMENT", "RULE_SL_COMMENT", "'type'", "':'", "';'", "'use'", "'='"
    };
    public static final int T__9=9;
    public static final int T__8=8;
    public static final int RULE_ID=4;
    public static final int RULE_WS=5;
    public static final int RULE_SL_COMMENT=7;
    public static final int T__11=11;
    public static final int RULE_ML_COMMENT=6;
    public static final int T__12=12;
    public static final int EOF=-1;
    public static final int T__10=10;

    // delegates
    // delegators


        public InternalReferenceEmptyScopeParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalReferenceEmptyScopeParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalReferenceEmptyScopeParser.tokenNames; }
    public String getGrammarFileName() { return "InternalReferenceEmptyScope.g"; }



     	private ReferenceEmptyScopeGrammarAccess grammarAccess;

        public InternalReferenceEmptyScopeParser(TokenStream input, ReferenceEmptyScopeGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected ReferenceEmptyScopeGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalReferenceEmptyScope.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalReferenceEmptyScope.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalReferenceEmptyScope.g:65:2: iv_ruleModel= ruleModel EOF
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
    // InternalReferenceEmptyScope.g:71:1: ruleModel returns [EObject current=null] : ( ( (lv_members_0_0= ruleType ) ) ( (lv_members_1_0= ruleFeature ) ) ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        EObject lv_members_0_0 = null;

        EObject lv_members_1_0 = null;



        	enterRule();

        try {
            // InternalReferenceEmptyScope.g:77:2: ( ( ( (lv_members_0_0= ruleType ) ) ( (lv_members_1_0= ruleFeature ) ) ) )
            // InternalReferenceEmptyScope.g:78:2: ( ( (lv_members_0_0= ruleType ) ) ( (lv_members_1_0= ruleFeature ) ) )
            {
            // InternalReferenceEmptyScope.g:78:2: ( ( (lv_members_0_0= ruleType ) ) ( (lv_members_1_0= ruleFeature ) ) )
            // InternalReferenceEmptyScope.g:79:3: ( (lv_members_0_0= ruleType ) ) ( (lv_members_1_0= ruleFeature ) )
            {
            // InternalReferenceEmptyScope.g:79:3: ( (lv_members_0_0= ruleType ) )
            // InternalReferenceEmptyScope.g:80:4: (lv_members_0_0= ruleType )
            {
            // InternalReferenceEmptyScope.g:80:4: (lv_members_0_0= ruleType )
            // InternalReferenceEmptyScope.g:81:5: lv_members_0_0= ruleType
            {

            					newCompositeNode(grammarAccess.getModelAccess().getMembersTypeParserRuleCall_0_0());
            				
            pushFollow(FOLLOW_3);
            lv_members_0_0=ruleType();

            state._fsp--;


            					if (current==null) {
            						current = createModelElementForParent(grammarAccess.getModelRule());
            					}
            					add(
            						current,
            						"members",
            						lv_members_0_0,
            						"audit.scopes.ReferenceEmptyScope.Type");
            					afterParserOrEnumRuleCall();
            				

            }


            }

            // InternalReferenceEmptyScope.g:98:3: ( (lv_members_1_0= ruleFeature ) )
            // InternalReferenceEmptyScope.g:99:4: (lv_members_1_0= ruleFeature )
            {
            // InternalReferenceEmptyScope.g:99:4: (lv_members_1_0= ruleFeature )
            // InternalReferenceEmptyScope.g:100:5: lv_members_1_0= ruleFeature
            {

            					newCompositeNode(grammarAccess.getModelAccess().getMembersFeatureParserRuleCall_1_0());
            				
            pushFollow(FOLLOW_2);
            lv_members_1_0=ruleFeature();

            state._fsp--;


            					if (current==null) {
            						current = createModelElementForParent(grammarAccess.getModelRule());
            					}
            					add(
            						current,
            						"members",
            						lv_members_1_0,
            						"audit.scopes.ReferenceEmptyScope.Feature");
            					afterParserOrEnumRuleCall();
            				

            }


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
    // $ANTLR end "ruleModel"


    // $ANTLR start "entryRuleType"
    // InternalReferenceEmptyScope.g:121:1: entryRuleType returns [EObject current=null] : iv_ruleType= ruleType EOF ;
    public final EObject entryRuleType() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleType = null;


        try {
            // InternalReferenceEmptyScope.g:121:45: (iv_ruleType= ruleType EOF )
            // InternalReferenceEmptyScope.g:122:2: iv_ruleType= ruleType EOF
            {
             newCompositeNode(grammarAccess.getTypeRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleType=ruleType();

            state._fsp--;

             current =iv_ruleType; 
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
    // $ANTLR end "entryRuleType"


    // $ANTLR start "ruleType"
    // InternalReferenceEmptyScope.g:128:1: ruleType returns [EObject current=null] : (otherlv_0= 'type' otherlv_1= ':' ( (lv_name_2_0= RULE_ID ) ) otherlv_3= ';' ) ;
    public final EObject ruleType() throws RecognitionException {
        EObject current = null;

        Token otherlv_0=null;
        Token otherlv_1=null;
        Token lv_name_2_0=null;
        Token otherlv_3=null;


        	enterRule();

        try {
            // InternalReferenceEmptyScope.g:134:2: ( (otherlv_0= 'type' otherlv_1= ':' ( (lv_name_2_0= RULE_ID ) ) otherlv_3= ';' ) )
            // InternalReferenceEmptyScope.g:135:2: (otherlv_0= 'type' otherlv_1= ':' ( (lv_name_2_0= RULE_ID ) ) otherlv_3= ';' )
            {
            // InternalReferenceEmptyScope.g:135:2: (otherlv_0= 'type' otherlv_1= ':' ( (lv_name_2_0= RULE_ID ) ) otherlv_3= ';' )
            // InternalReferenceEmptyScope.g:136:3: otherlv_0= 'type' otherlv_1= ':' ( (lv_name_2_0= RULE_ID ) ) otherlv_3= ';'
            {
            otherlv_0=(Token)match(input,8,FOLLOW_4); 

            			newLeafNode(otherlv_0, grammarAccess.getTypeAccess().getTypeKeyword_0());
            		
            otherlv_1=(Token)match(input,9,FOLLOW_5); 

            			newLeafNode(otherlv_1, grammarAccess.getTypeAccess().getColonKeyword_1());
            		
            // InternalReferenceEmptyScope.g:144:3: ( (lv_name_2_0= RULE_ID ) )
            // InternalReferenceEmptyScope.g:145:4: (lv_name_2_0= RULE_ID )
            {
            // InternalReferenceEmptyScope.g:145:4: (lv_name_2_0= RULE_ID )
            // InternalReferenceEmptyScope.g:146:5: lv_name_2_0= RULE_ID
            {
            lv_name_2_0=(Token)match(input,RULE_ID,FOLLOW_6); 

            					newLeafNode(lv_name_2_0, grammarAccess.getTypeAccess().getNameIDTerminalRuleCall_2_0());
            				

            					if (current==null) {
            						current = createModelElement(grammarAccess.getTypeRule());
            					}
            					setWithLastConsumed(
            						current,
            						"name",
            						lv_name_2_0,
            						"audit.scopes.ReferenceEmptyScope.ID");
            				

            }


            }

            otherlv_3=(Token)match(input,10,FOLLOW_2); 

            			newLeafNode(otherlv_3, grammarAccess.getTypeAccess().getSemicolonKeyword_3());
            		

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
    // $ANTLR end "ruleType"


    // $ANTLR start "entryRuleFeature"
    // InternalReferenceEmptyScope.g:170:1: entryRuleFeature returns [EObject current=null] : iv_ruleFeature= ruleFeature EOF ;
    public final EObject entryRuleFeature() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleFeature = null;


        try {
            // InternalReferenceEmptyScope.g:170:48: (iv_ruleFeature= ruleFeature EOF )
            // InternalReferenceEmptyScope.g:171:2: iv_ruleFeature= ruleFeature EOF
            {
             newCompositeNode(grammarAccess.getFeatureRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleFeature=ruleFeature();

            state._fsp--;

             current =iv_ruleFeature; 
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
    // $ANTLR end "entryRuleFeature"


    // $ANTLR start "ruleFeature"
    // InternalReferenceEmptyScope.g:177:1: ruleFeature returns [EObject current=null] : (otherlv_0= 'use' otherlv_1= ':' ( (lv_name_2_0= RULE_ID ) ) otherlv_3= '=' ( ( ruleQName ) ) otherlv_5= ';' ) ;
    public final EObject ruleFeature() throws RecognitionException {
        EObject current = null;

        Token otherlv_0=null;
        Token otherlv_1=null;
        Token lv_name_2_0=null;
        Token otherlv_3=null;
        Token otherlv_5=null;


        	enterRule();

        try {
            // InternalReferenceEmptyScope.g:183:2: ( (otherlv_0= 'use' otherlv_1= ':' ( (lv_name_2_0= RULE_ID ) ) otherlv_3= '=' ( ( ruleQName ) ) otherlv_5= ';' ) )
            // InternalReferenceEmptyScope.g:184:2: (otherlv_0= 'use' otherlv_1= ':' ( (lv_name_2_0= RULE_ID ) ) otherlv_3= '=' ( ( ruleQName ) ) otherlv_5= ';' )
            {
            // InternalReferenceEmptyScope.g:184:2: (otherlv_0= 'use' otherlv_1= ':' ( (lv_name_2_0= RULE_ID ) ) otherlv_3= '=' ( ( ruleQName ) ) otherlv_5= ';' )
            // InternalReferenceEmptyScope.g:185:3: otherlv_0= 'use' otherlv_1= ':' ( (lv_name_2_0= RULE_ID ) ) otherlv_3= '=' ( ( ruleQName ) ) otherlv_5= ';'
            {
            otherlv_0=(Token)match(input,11,FOLLOW_4); 

            			newLeafNode(otherlv_0, grammarAccess.getFeatureAccess().getUseKeyword_0());
            		
            otherlv_1=(Token)match(input,9,FOLLOW_5); 

            			newLeafNode(otherlv_1, grammarAccess.getFeatureAccess().getColonKeyword_1());
            		
            // InternalReferenceEmptyScope.g:193:3: ( (lv_name_2_0= RULE_ID ) )
            // InternalReferenceEmptyScope.g:194:4: (lv_name_2_0= RULE_ID )
            {
            // InternalReferenceEmptyScope.g:194:4: (lv_name_2_0= RULE_ID )
            // InternalReferenceEmptyScope.g:195:5: lv_name_2_0= RULE_ID
            {
            lv_name_2_0=(Token)match(input,RULE_ID,FOLLOW_7); 

            					newLeafNode(lv_name_2_0, grammarAccess.getFeatureAccess().getNameIDTerminalRuleCall_2_0());
            				

            					if (current==null) {
            						current = createModelElement(grammarAccess.getFeatureRule());
            					}
            					setWithLastConsumed(
            						current,
            						"name",
            						lv_name_2_0,
            						"audit.scopes.ReferenceEmptyScope.ID");
            				

            }


            }

            otherlv_3=(Token)match(input,12,FOLLOW_5); 

            			newLeafNode(otherlv_3, grammarAccess.getFeatureAccess().getEqualsSignKeyword_3());
            		
            // InternalReferenceEmptyScope.g:215:3: ( ( ruleQName ) )
            // InternalReferenceEmptyScope.g:216:4: ( ruleQName )
            {
            // InternalReferenceEmptyScope.g:216:4: ( ruleQName )
            // InternalReferenceEmptyScope.g:217:5: ruleQName
            {

            					if (current==null) {
            						current = createModelElement(grammarAccess.getFeatureRule());
            					}
            				

            					newCompositeNode(grammarAccess.getFeatureAccess().getTargetTypeCrossReference_4_0());
            				
            pushFollow(FOLLOW_6);
            ruleQName();

            state._fsp--;


            					afterParserOrEnumRuleCall();
            				

            }


            }

            otherlv_5=(Token)match(input,10,FOLLOW_2); 

            			newLeafNode(otherlv_5, grammarAccess.getFeatureAccess().getSemicolonKeyword_5());
            		

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
    // $ANTLR end "ruleFeature"


    // $ANTLR start "entryRuleQName"
    // InternalReferenceEmptyScope.g:239:1: entryRuleQName returns [String current=null] : iv_ruleQName= ruleQName EOF ;
    public final String entryRuleQName() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleQName = null;



        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalReferenceEmptyScope.g:241:2: (iv_ruleQName= ruleQName EOF )
            // InternalReferenceEmptyScope.g:242:2: iv_ruleQName= ruleQName EOF
            {
             newCompositeNode(grammarAccess.getQNameRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleQName=ruleQName();

            state._fsp--;

             current =iv_ruleQName.getText(); 
            match(input,EOF,FOLLOW_2); 

            }

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
    // $ANTLR end "entryRuleQName"


    // $ANTLR start "ruleQName"
    // InternalReferenceEmptyScope.g:251:1: ruleQName returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : this_ID_0= RULE_ID ;
    public final AntlrDatatypeRuleToken ruleQName() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        Token this_ID_0=null;


        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalReferenceEmptyScope.g:258:2: (this_ID_0= RULE_ID )
            // InternalReferenceEmptyScope.g:259:2: this_ID_0= RULE_ID
            {
            this_ID_0=(Token)match(input,RULE_ID,FOLLOW_2); 

            		current.merge(this_ID_0);
            	

            		newLeafNode(this_ID_0, grammarAccess.getQNameAccess().getIDTerminalRuleCall());
            	

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
    // $ANTLR end "ruleQName"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000000L});
    public static final BitSet FOLLOW_2 = new BitSet(new long[]{0x0000000000000002L});
    public static final BitSet FOLLOW_3 = new BitSet(new long[]{0x0000000000000800L});
    public static final BitSet FOLLOW_4 = new BitSet(new long[]{0x0000000000000200L});
    public static final BitSet FOLLOW_5 = new BitSet(new long[]{0x0000000000000010L});
    public static final BitSet FOLLOW_6 = new BitSet(new long[]{0x0000000000000400L});
    public static final BitSet FOLLOW_7 = new BitSet(new long[]{0x0000000000001000L});

}