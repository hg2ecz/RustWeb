use crate::{
    ActionFunction, ComponentFunction, EnumDef, FormSchema, LayoutFunction, Model, PageFunction,
    QueryFunction, ResourceUse, Route,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Program {
    pub enums: Vec<EnumDef>,
    pub models: Vec<Model>,
    pub queries: Vec<QueryFunction>,
    pub pages: Vec<PageFunction>,
    pub actions: Vec<ActionFunction>,
    pub routes: Vec<Route>,
    pub forms: Vec<FormSchema>,
    pub components: Vec<ComponentFunction>,
    pub layouts: Vec<LayoutFunction>,
    pub resource_uses: Vec<ResourceUse>,
}

impl Program {
    pub fn enum_by_name(&self, name: &str) -> Option<(u16, &EnumDef)> {
        self.enums
            .iter()
            .enumerate()
            .find(|(_, v)| v.name == name)
            .and_then(|(i, v)| u16::try_from(i).ok().map(|id| (id, v)))
    }

    pub fn enum_by_id(&self, id: u16) -> Option<&EnumDef> {
        self.enums.get(id as usize)
    }

    pub fn model(&self, name: &str) -> Option<&Model> {
        self.models.iter().find(|v| v.name == name)
    }

    pub fn form(&self, name: &str) -> Option<&FormSchema> {
        self.forms.iter().find(|v| v.name == name)
    }

    pub fn component(&self, name: &str) -> Option<&ComponentFunction> {
        self.components.iter().find(|v| v.name == name)
    }

    pub fn layout(&self, name: &str) -> Option<&LayoutFunction> {
        self.layouts.iter().find(|v| v.name == name)
    }

    pub fn query(&self, name: &str) -> Option<&QueryFunction> {
        self.queries.iter().find(|v| v.name == name)
    }

    pub fn page(&self, name: &str) -> Option<&PageFunction> {
        self.pages.iter().find(|v| v.name == name)
    }

    pub fn action(&self, name: &str) -> Option<&ActionFunction> {
        self.actions.iter().find(|v| v.name == name)
    }
}
