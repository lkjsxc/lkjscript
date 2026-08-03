macro_rules! lifecycle_services {
    () => {
        fn publish_structural_owner(
            &mut self,
            owner: NativeStructuralOwner,
            storage: StructuralStorageRoute,
        ) -> Result<NativeStructuralOwner, NativeServiceError> {
            self.structural.publish_owner(owner, storage)
        }

        fn copy_structural(
            &mut self,
            owner: NativeStructuralOwner,
        ) -> Result<NativeStructuralOwner, NativeServiceError> {
            self.structural.copy_owner(owner)
        }

        fn move_structural(
            &mut self,
            owner: NativeStructuralOwner,
        ) -> Result<NativeStructuralOwner, NativeServiceError> {
            self.structural.move_owner(owner)
        }

        fn independent_structural_owner(
            &mut self,
            witness: u16,
            key: u64,
        ) -> Result<u64, NativeServiceError> {
            self.acquire_witness_owner(witness, key)
        }

        fn dispose_structural_owner(
            &mut self,
            witness: u16,
            key: u64,
        ) -> Result<(), NativeServiceError> {
            self.dispose_witness_owner(witness, key)
        }

        fn compare_structural_values(
            &mut self,
            witness: u16,
            left: u64,
            right: u64,
        ) -> Result<bool, NativeServiceError> {
            self.compare_witness_values(witness, left, right)
        }

        fn drop_structural(
            &mut self,
            owner: NativeStructuralOwner,
        ) -> Result<(), NativeServiceError> {
            self.structural.drop_owner(owner)
        }
    };
}

pub(super) use lifecycle_services;
