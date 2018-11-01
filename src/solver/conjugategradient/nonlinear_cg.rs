// Copyright 2018 Stefan Kroboth
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! # Nonlinear Conjugate Gradient Method
//!
//! TODO: Proper documentation.
//!
//! Important TODO: Find out which line search should be the default choice. Also try to replicate
//! CG_DESCENT.
//!
// //!
// //! # Example
// //!
// //! ```rust
// //! todo
// //! ```

use prelude::*;
use solver::conjugategradient::{FletcherReeves, HestenesStiefel, PolakRibiere, PolakRibierePlus};
use solver::linesearch::HagerZhangLineSearch;
use std;
use std::default::Default;

/// Nonlinear Conjugate Gradient struct
#[derive(ArgminSolver)]
pub struct NonlinearConjugateGradient<'a, T>
where
    T: 'a
        + Clone
        + Default
        + ArgminSub<T>
        + ArgminAdd<T>
        + ArgminScale<f64>
        + ArgminNorm<f64>
        + ArgminDot<T, f64>
        + ArgminScaledAdd<T, f64>
        + ArgminScaledSub<T, f64>,
{
    /// p
    p: T,
    /// beta
    beta: f64,
    /// line search
    linesearch: Box<ArgminLineSearch<Parameters = T, OperatorOutput = f64, Hessian = ()> + 'a>,
    /// beta update method
    beta_method: Box<ArgminNLCGBetaUpdate<T> + 'a>,
    /// base
    base: ArgminBase<'a, T, f64, ()>,
}

impl<'a, T> NonlinearConjugateGradient<'a, T>
where
    T: 'a
        + Clone
        + Default
        + ArgminSub<T>
        + ArgminAdd<T>
        + ArgminScale<f64>
        + ArgminNorm<f64>
        + ArgminDot<T, f64>
        + ArgminScaledAdd<T, f64>
        + ArgminScaledSub<T, f64>,
{
    /// Constructor (Polak Ribiere Conjugate Gradient (PR-CG))
    ///
    /// Parameters:
    ///
    /// `cost_function`: cost function
    /// `init_param`: Initial parameter vector
    pub fn new(
        operator: Box<ArgminOperator<Parameters = T, OperatorOutput = f64, Hessian = ()>>,
        init_param: T,
    ) -> Result<Self, Error> {
        let linesearch = HagerZhangLineSearch::new(operator.clone());
        // let beta_method = FletcherReeves::new();
        let beta_method = PolakRibiere::new();
        // let beta_method = PolakRibierePlus::new();
        Ok(NonlinearConjugateGradient {
            p: T::default(),
            beta: std::f64::NAN,
            linesearch: Box::new(linesearch),
            beta_method: Box::new(beta_method),
            base: ArgminBase::new(operator, init_param),
        })
    }

    /// New PolakRibiere CG (PR-CG)
    pub fn new_pr(
        operator: Box<ArgminOperator<Parameters = T, OperatorOutput = f64, Hessian = ()>>,
        init_param: T,
    ) -> Result<Self, Error> {
        Self::new(operator, init_param)
    }

    /// New PolakRibierePlus CG (PR+-CG)
    pub fn new_prplus(
        operator: Box<ArgminOperator<Parameters = T, OperatorOutput = f64, Hessian = ()>>,
        init_param: T,
    ) -> Result<Self, Error> {
        let mut s = Self::new(operator, init_param)?;
        let beta_method = PolakRibierePlus::new();
        s.set_beta_update(Box::new(beta_method));
        Ok(s)
    }

    /// New FletcherReeves CG (FR-CG)
    pub fn new_fr(
        operator: Box<ArgminOperator<Parameters = T, OperatorOutput = f64, Hessian = ()>>,
        init_param: T,
    ) -> Result<Self, Error> {
        let mut s = Self::new(operator, init_param)?;
        let beta_method = FletcherReeves::new();
        s.set_beta_update(Box::new(beta_method));
        Ok(s)
    }

    /// New HestenesStiefel CG (HS-CG)
    pub fn new_hs(
        operator: Box<ArgminOperator<Parameters = T, OperatorOutput = f64, Hessian = ()>>,
        init_param: T,
    ) -> Result<Self, Error> {
        let mut s = Self::new(operator, init_param)?;
        let beta_method = HestenesStiefel::new();
        s.set_beta_update(Box::new(beta_method));
        Ok(s)
    }

    /// Specify line search method
    pub fn set_linesearch(
        &mut self,
        linesearch: Box<ArgminLineSearch<Parameters = T, OperatorOutput = f64, Hessian = ()> + 'a>,
    ) -> &mut Self {
        self.linesearch = linesearch;
        self
    }

    /// Specify beta update method
    pub fn set_beta_update(&mut self, beta_method: Box<ArgminNLCGBetaUpdate<T> + 'a>) -> &mut Self {
        self.beta_method = beta_method;
        self
    }
}

impl<'a, T> ArgminNextIter for NonlinearConjugateGradient<'a, T>
where
    T: 'a
        + Clone
        + Default
        + ArgminSub<T>
        + ArgminAdd<T>
        + ArgminScale<f64>
        + ArgminNorm<f64>
        + ArgminDot<T, f64>
        + ArgminScaledAdd<T, f64>
        + ArgminScaledSub<T, f64>,
{
    type Parameters = T;
    type OperatorOutput = f64;
    type Hessian = ();

    fn init(&mut self) -> Result<(), Error> {
        let param = self.cur_param();
        let cost = self.apply(&param)?;
        let grad = self.gradient(&param)?;
        self.p = grad.scale(-1.0);
        self.set_cur_cost(cost);
        self.set_cur_grad(grad);
        Ok(())
    }

    /// Perform one iteration of SA algorithm
    fn next_iter(&mut self) -> Result<ArgminIterationData<Self::Parameters>, Error> {
        // reset line search
        self.linesearch.base_reset();

        let xk = self.cur_param();
        let grad = self.cur_grad();
        let pk = self.p.clone();
        let cur_cost = self.cur_cost();

        // Linesearch
        self.linesearch.set_initial_parameter(xk);
        self.linesearch.set_search_direction(pk.clone());
        self.linesearch.set_initial_gradient(grad.clone());
        self.linesearch.set_initial_cost(cur_cost);

        self.linesearch.run_fast()?;

        let xk1 = self.linesearch.result().param;

        // Update of beta
        let new_grad = self.gradient(&xk1)?;

        self.beta = self.beta_method.update(&grad, &new_grad, &pk);

        // Update of p
        self.p = new_grad.scale(-1.0).add(self.p.scale(self.beta));

        // Housekeeping
        self.set_cur_param(xk1.clone());
        self.set_cur_grad(new_grad);
        let cost = self.apply(&xk1)?;
        self.set_cur_cost(cost);

        let mut out = ArgminIterationData::new(xk1, cost);
        out.add_kv(make_kv!(
                "beta" => self.beta;
            ));
        Ok(out)
    }
}
